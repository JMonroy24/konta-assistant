use std::sync::Arc;
use std::time::Duration;

use chrono::{NaiveDate, Utc};
use domain::{EstadoConflicto, Operacion, TipoOperacion};
use rust_decimal_macros::dec;

use super::{ContadoresSync, DetectarConflictos, SincronizarCola};
use crate::ports::{
    AplicadorSyncPort, ColaSyncPort, ConflictoRepository, ItemSync, ResultadoAplicacionSync,
};
use crate::testing::{AlmacenEnMemoria, AplicadorSyncEnMemoria, EsperaSyncEnMemoria};

fn elemento(id: &str) -> ItemSync {
    ItemSync {
        id: id.into(),
        endpoint: "/operaciones".into(),
        payload: serde_json::json!({"id": id}),
        timestamp: Utc::now(),
    }
}

fn operacion(id: &str, rev: &str, base: rust_decimal::Decimal) -> Operacion {
    Operacion {
        id: id.into(),
        rev: Some(rev.into()),
        tipo: TipoOperacion::Venta,
        fecha: NaiveDate::from_ymd_opt(2026, 11, 5).expect("fecha válida"),
        descripcion: "Venta de prueba".into(),
        base,
        iva: dec!(13),
        total: base + dec!(13),
        contraparte: "Cliente".into(),
        creado_por: "usuario".into(),
        origen: "local".into(),
        creado_en: Utc::now(),
    }
}

#[tokio::test]
async fn reenviar_el_mismo_uuid_no_duplica_la_aplicacion() {
    let cola = Arc::new(AlmacenEnMemoria::nuevo());
    let aplicador = Arc::new(AplicadorSyncEnMemoria::default());
    let espera = Arc::new(EsperaSyncEnMemoria::default());
    let contadores = Arc::new(ContadoresSync::default());
    let caso = SincronizarCola::new(cola.clone(), aplicador.clone(), espera, contadores.clone());
    let item = elemento("uuid-1");

    cola.encolar(item.clone()).await.expect("encolar");
    caso.ejecutar().await.expect("primera sincronización");
    cola.encolar(item).await.expect("reenviar");
    caso.ejecutar().await.expect("segunda sincronización");

    assert_eq!(
        aplicador
            .cantidad_aplicaciones("uuid-1")
            .expect("contar aplicaciones"),
        1
    );
    assert_eq!(contadores.resumen().sincronizadas, 1);
    assert!(cola.pendientes().await.expect("pendientes").is_empty());
}

#[tokio::test]
async fn falla_transitoria_reintenta_con_backoff_exponencial() {
    let cola = Arc::new(AlmacenEnMemoria::nuevo());
    let aplicador = Arc::new(AplicadorSyncEnMemoria::default());
    let espera = Arc::new(EsperaSyncEnMemoria::default());
    let contadores = Arc::new(ContadoresSync::default());
    let caso = SincronizarCola::new(
        cola.clone(),
        aplicador.clone(),
        espera.clone(),
        contadores.clone(),
    );
    cola.encolar(elemento("uuid-transitorio"))
        .await
        .expect("encolar");
    aplicador
        .fallar_siguientes_intentos("uuid-transitorio", 2)
        .expect("configurar fallos");

    caso.ejecutar().await.expect("debe recuperarse");

    assert_eq!(aplicador.intentos("uuid-transitorio").expect("intentos"), 3);
    assert_eq!(contadores.resumen().reintentos, 2);
    assert_eq!(
        espera.duraciones().expect("esperas"),
        vec![Duration::from_millis(100), Duration::from_millis(200)]
    );
}

#[tokio::test]
async fn limita_a_cinco_intentos_y_deja_pendiente_el_elemento_fallido() {
    let cola = Arc::new(AlmacenEnMemoria::nuevo());
    let aplicador = Arc::new(AplicadorSyncEnMemoria::default());
    let espera = Arc::new(EsperaSyncEnMemoria::default());
    let contadores = Arc::new(ContadoresSync::default());
    let caso = SincronizarCola::new(
        cola.clone(),
        aplicador.clone(),
        espera.clone(),
        contadores.clone(),
    );
    cola.encolar(elemento("uuid-falla")).await.expect("encolar");
    aplicador
        .fallar_siguientes_intentos("uuid-falla", 5)
        .expect("configurar fallos");

    assert!(caso.ejecutar().await.is_err());

    assert_eq!(aplicador.intentos("uuid-falla").expect("intentos"), 5);
    assert_eq!(contadores.resumen().reintentos, 4);
    assert_eq!(
        espera.duraciones().expect("esperas"),
        vec![
            Duration::from_millis(100),
            Duration::from_millis(200),
            Duration::from_millis(400),
            Duration::from_millis(800)
        ]
    );
    assert_eq!(cola.pendientes().await.expect("pendientes").len(), 1);
}

#[tokio::test]
async fn conflicto_se_guarda_pendiente_sin_fusionar_importes() {
    let almacen = Arc::new(AlmacenEnMemoria::nuevo());
    let contadores = Arc::new(ContadoresSync::default());
    let caso = DetectarConflictos::new(almacen.clone(), contadores.clone());
    let ganadora = operacion("op-1", "1-a", dec!(100));
    let perdedora = operacion("op-1", "1-b", dec!(120));

    let conflicto = caso
        .ejecutar(&ganadora, std::slice::from_ref(&perdedora))
        .await
        .expect("detectar conflicto")
        .expect("conflicto presente");

    assert_eq!(conflicto.estado, EstadoConflicto::Pendiente);
    assert_eq!(conflicto.campos_en_conflicto, vec!["base", "total"]);
    assert_eq!(conflicto.revisiones_en_conflicto[0].base, dec!(120));
    assert_eq!(ganadora.base, dec!(100));
    assert_eq!(perdedora.base, dec!(120));
    assert_eq!(
        almacen
            .listar_pendientes()
            .await
            .expect("conflictos pendientes")
            .len(),
        1
    );
    assert_eq!(contadores.resumen().conflictos, 1);
}

#[tokio::test]
async fn elemento_en_conflicto_incrementa_contador_y_sale_de_la_cola() {
    let cola = Arc::new(AlmacenEnMemoria::nuevo());
    let aplicador = Arc::new(AplicadorSyncEnMemoria::default());
    let contadores = Arc::new(ContadoresSync::default());
    let caso = SincronizarCola::new(
        cola.clone(),
        aplicador.clone(),
        Arc::new(EsperaSyncEnMemoria::default()),
        contadores.clone(),
    );
    cola.encolar(elemento("uuid-conflicto"))
        .await
        .expect("encolar");
    aplicador
        .marcar_como_conflicto("uuid-conflicto")
        .expect("configurar conflicto");

    let resultado = caso.ejecutar().await.expect("procesar conflicto");

    assert_eq!(resultado.conflictos, 1);
    assert_eq!(resultado.sincronizadas, 0);
    assert!(cola.pendientes().await.expect("pendientes").is_empty());
    assert_eq!(
        aplicador
            .aplicar_idempotente(&elemento("uuid-conflicto"))
            .await
            .expect("reaplicar de forma idempotente"),
        ResultadoAplicacionSync::YaAplicada
    );
}
