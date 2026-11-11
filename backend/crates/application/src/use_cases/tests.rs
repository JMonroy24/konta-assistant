use std::sync::Arc;

use chrono::NaiveDate;
use domain::{
    DomainError, EstadoDeclaracion, EstadoFactura, EstadoRecordatorio, Operacion, Periodo,
    ReglaFiscal, TipoImpuesto, TipoOperacion,
};
use rust_decimal_macros::dec;
use serde_json::json;

use super::*;
use crate::ports::{
    DeclaracionRepository, FacturaRepository, OperacionRepository, RecordatorioRepository,
    ReglaFiscalRepository,
};
use crate::testing::{
    AlmacenEnMemoria, CalculadoraFiscalEnMemoria, HaciendaSimuladorAcepta, HaciendaSimuladorFalla,
    HaciendaSimuladorRechaza, RelojFijo,
};

fn fecha(texto: &str) -> NaiveDate {
    NaiveDate::parse_from_str(texto, "%Y-%m-%d").expect("fecha de prueba válida")
}

fn periodo(texto: &str) -> Periodo {
    Periodo::parse(texto).expect("período de prueba válido")
}

fn regla(codigo: &str, parametros: serde_json::Value) -> ReglaFiscal {
    ReglaFiscal {
        id: format!("regla:{codigo}:v1"),
        rev: None,
        tipo: "regla_fiscal".into(),
        codigo: codigo.into(),
        vigencia_desde: fecha("2020-01-01"),
        vigencia_hasta: None,
        fuente_normativa: "prueba".into(),
        version: 1,
        verificada: true,
        parametros,
    }
}

fn reglas_base() -> Vec<ReglaFiscal> {
    vec![
        regla(CODIGO_IVA_GENERAL, json!({ "tasa": "0.13" })),
        regla(CODIGO_PAGO_CUENTA, json!({ "tasa": "0.0175" })),
        regla(
            CODIGO_PLAZO_IVA_F07,
            json!({ "modo": "dias_habiles_mes_siguiente", "dias": 10 }),
        ),
        regla(
            CODIGO_PLAZO_PAGO_CUENTA_F14,
            json!({ "modo": "dias_habiles_mes_siguiente", "dias": 10 }),
        ),
        regla(
            CODIGO_PLAZO_RENTA_ANUAL_F11,
            json!({ "modo": "fecha_fija_anual", "mes": 4, "dia": 30 }),
        ),
    ]
}

fn operacion(
    id: &str,
    tipo: TipoOperacion,
    fecha_texto: &str,
    base: rust_decimal::Decimal,
    iva: rust_decimal::Decimal,
) -> Operacion {
    Operacion {
        id: id.into(),
        rev: None,
        tipo,
        fecha: fecha(fecha_texto),
        descripcion: "Operación de prueba".into(),
        base,
        iva,
        total: base + iva,
        contraparte: "Contraparte".into(),
        creado_por: "usr1".into(),
        origen: "local".into(),
        creado_en: chrono::Utc::now(),
    }
}

async fn almacen_con_reglas() -> Arc<AlmacenEnMemoria> {
    let almacen = AlmacenEnMemoria::nuevo();
    for regla in reglas_base() {
        almacen.guardar_regla(regla).await.expect("guardar regla");
    }
    Arc::new(almacen)
}

#[tokio::test]
async fn registrar_operacion_valida_y_rechaza_iva_invalido() {
    let almacen = almacen_con_reglas().await;
    let caso = RegistrarOperacion::new(
        almacen.clone(),
        almacen.clone(),
        Arc::new(CalculadoraFiscalEnMemoria::nuevo()),
    );

    let guardada = caso
        .ejecutar(
            "negocio_001",
            operacion(
                "op-1",
                TipoOperacion::Venta,
                "2026-11-05",
                dec!(100),
                dec!(13),
            ),
        )
        .await
        .expect("operación válida");
    assert_eq!(guardada.id, "op-1");

    let resultado = caso
        .ejecutar(
            "negocio_001",
            operacion(
                "op-2",
                TipoOperacion::Venta,
                "2026-11-05",
                dec!(100),
                dec!(5),
            ),
        )
        .await;
    assert!(matches!(resultado, Err(DomainError::ValidacionError(_))));
    assert_eq!(
        almacen
            .listar_por_periodo("negocio_001", "2026-11")
            .await
            .expect("listar operaciones")
            .len(),
        1
    );
}

#[tokio::test]
async fn consultar_resumen_calcula_impuestos_del_periodo() {
    let almacen = almacen_con_reglas().await;
    for (id, tipo, fecha, base, iva) in [
        (
            "op-1",
            TipoOperacion::Venta,
            "2026-11-05",
            dec!(100),
            dec!(13),
        ),
        (
            "op-2",
            TipoOperacion::Compra,
            "2026-11-06",
            dec!(40),
            dec!(5.2),
        ),
    ] {
        almacen
            .guardar_operacion("negocio_001", operacion(id, tipo, fecha, base, iva))
            .await
            .expect("guardar operación");
    }

    let caso = ConsultarResumenImpuestos::new(
        almacen.clone(),
        almacen,
        Arc::new(CalculadoraFiscalEnMemoria::nuevo()),
    );
    let resumen = caso
        .ejecutar("negocio_001", periodo("2026-11"), None)
        .await
        .expect("resumen válido");

    assert_eq!(resumen.iva_a_pagar, dec!(7.8));
    assert_eq!(resumen.pago_a_cuenta, dec!(1.75));
}

#[tokio::test]
async fn generar_borrador_es_idempotente_por_periodo_y_tipo() {
    let almacen = almacen_con_reglas().await;
    let caso = GenerarBorradorDeclaracion::new(
        almacen.clone(),
        almacen.clone(),
        almacen.clone(),
        Arc::new(CalculadoraFiscalEnMemoria::nuevo()),
    );
    let periodo = periodo("2026-11");
    let primera = caso
        .ejecutar("negocio_001", periodo, TipoImpuesto::IvaF07)
        .await
        .expect("crear borrador");
    let repetida = caso
        .ejecutar("negocio_001", periodo, TipoImpuesto::IvaF07)
        .await
        .expect("recuperar borrador");
    let otra = caso
        .ejecutar("negocio_001", periodo, TipoImpuesto::PagoCuentaF14)
        .await
        .expect("crear segundo tipo");

    assert_eq!(primera.id, repetida.id);
    assert_eq!(repetida.estado, EstadoDeclaracion::Borrador);
    assert_ne!(primera.id, otra.id);
    assert!(almacen
        .obtener_declaracion("negocio_001", "2026-11", TipoImpuesto::IvaF07)
        .await
        .expect("buscar borrador")
        .is_some());
    assert!(almacen
        .obtener_declaracion("negocio_001", "2026-11", TipoImpuesto::PagoCuentaF14)
        .await
        .expect("buscar segundo borrador")
        .is_some());
}

#[tokio::test]
async fn listar_recordatorios_persiste_obligaciones_pendientes() {
    let almacen = almacen_con_reglas().await;
    let caso = ListarRecordatorios::new(
        almacen.clone(),
        almacen.clone(),
        Arc::new(CalculadoraFiscalEnMemoria::nuevo()),
    );
    let recordatorios = caso
        .ejecutar("negocio_001", periodo("2026-11"), &[])
        .await
        .expect("generar recordatorios");

    assert_eq!(recordatorios.len(), 3);
    assert!(recordatorios
        .iter()
        .all(|recordatorio| recordatorio.estado == EstadoRecordatorio::Pendiente));
    assert_eq!(
        almacen
            .listar_proximos("negocio_001")
            .await
            .expect("listar recordatorios")
            .len(),
        3
    );
}

#[tokio::test]
async fn emitir_factura_refleja_respuesta_del_simulador() {
    for (simulador, estado) in [
        (
            Arc::new(HaciendaSimuladorAcepta) as Arc<dyn crate::ports::HaciendaSimulatorPort>,
            EstadoFactura::AceptadoSimulado,
        ),
        (
            Arc::new(HaciendaSimuladorRechaza) as Arc<dyn crate::ports::HaciendaSimulatorPort>,
            EstadoFactura::Rechazado,
        ),
    ] {
        let almacen = Arc::new(AlmacenEnMemoria::nuevo());
        let caso = EmitirFactura::new(almacen, simulador, Arc::new(RelojFijo::default()));
        let factura = caso
            .ejecutar("negocio_001", "venta-1", "FAC-001")
            .await
            .expect("respuesta del simulador");
        assert_eq!(factura.estado_validacion, estado);
        assert!(factura.mensaje_simulador.is_some());
    }
}

#[tokio::test]
async fn emitir_factura_sin_respuesta_permanece_pendiente() {
    let almacen = Arc::new(AlmacenEnMemoria::nuevo());
    let caso = EmitirFactura::new(
        almacen.clone(),
        Arc::new(HaciendaSimuladorFalla),
        Arc::new(RelojFijo::default()),
    );
    assert!(caso
        .ejecutar("negocio_001", "venta-1", "FAC-001")
        .await
        .is_err());

    let guardada = almacen
        .obtener_factura("negocio_001", "factura:negocio_001:venta-1")
        .await
        .expect("buscar factura")
        .expect("factura pendiente persistida");
    assert_eq!(guardada.estado_validacion, EstadoFactura::Pendiente);
}
