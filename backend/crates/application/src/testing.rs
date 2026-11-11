//! Implementaciones en memoria de todos los puertos, para que el equipo pueda
//! desarrollar y probar sin CouchDB, sin red y con un reloj fijo.
//!
//! Solo se compila con la feature `testing`:
//!
//! ```toml
//! application = { path = "../application", features = ["testing"] }
//! ```

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, MutexGuard};

use async_trait::async_trait;
use chrono::{DateTime, Datelike, NaiveDate, Utc, Weekday};
use domain::{
    Conflicto, Declaracion, DomainError, EstadoConflicto, Factura, ObligacionFiscal, Operacion,
    Periodo, Recordatorio, ReglaFiscal, ResumenImpuestos, TipoImpuesto, TipoObligacion,
    TipoOperacion,
};
use rust_decimal::{Decimal, RoundingStrategy};
use serde_json::json;

use crate::ports::{
    AplicadorSyncPort, CalculadoraFiscalPort, ColaSyncPort, ConflictoRepository,
    DeclaracionRepository, EsperaSyncPort, FacturaRepository, HaciendaSimulatorPort, ItemSync,
    NotificacionPort, OperacionRepository, RecordatorioRepository, ReglaFiscalRepository,
    RelojPort, ResultadoAplicacionSync, SyncStatusPort,
};

fn bloquear<T>(m: &Mutex<T>) -> Result<MutexGuard<'_, T>, DomainError> {
    m.lock()
        .map_err(|_| DomainError::ValidacionError("Mutex envenenado".into()))
}

#[derive(Default)]
struct ColaInterna {
    pendientes: Vec<ItemSync>,
    aplicadas: HashSet<String>,
}

/// Almacén en memoria que implementa todos los puertos de persistencia y de
/// infraestructura síncrona.
#[derive(Default)]
pub struct AlmacenEnMemoria {
    operaciones: Mutex<HashMap<String, Vec<Operacion>>>,
    facturas: Mutex<HashMap<String, Vec<Factura>>>,
    declaraciones: Mutex<HashMap<String, Vec<Declaracion>>>,
    recordatorios: Mutex<Vec<Recordatorio>>,
    reglas: Mutex<Vec<ReglaFiscal>>,
    conflictos: Mutex<Vec<Conflicto>>,
    cola: Mutex<ColaInterna>,
    notificaciones: Mutex<Vec<Recordatorio>>,
}

impl AlmacenEnMemoria {
    pub fn nuevo() -> Self {
        Self::default()
    }

    /// Recordatorios notificados, útil para aserciones en pruebas.
    pub fn notificaciones_enviadas(&self) -> Result<Vec<Recordatorio>, DomainError> {
        Ok(bloquear(&self.notificaciones)?.clone())
    }
}

#[async_trait]
impl OperacionRepository for AlmacenEnMemoria {
    async fn guardar_operacion(
        &self,
        negocio_id: &str,
        operacion: Operacion,
    ) -> Result<Operacion, DomainError> {
        let mut mapa = bloquear(&self.operaciones)?;
        let lista = mapa.entry(negocio_id.to_string()).or_default();
        match lista.iter_mut().find(|o| o.id == operacion.id) {
            Some(existente) => *existente = operacion.clone(),
            None => lista.push(operacion.clone()),
        }
        Ok(operacion)
    }

    async fn obtener_operacion(
        &self,
        negocio_id: &str,
        id: &str,
    ) -> Result<Option<Operacion>, DomainError> {
        let mapa = bloquear(&self.operaciones)?;
        Ok(mapa
            .get(negocio_id)
            .and_then(|lista| lista.iter().find(|o| o.id == id).cloned()))
    }

    async fn listar_por_periodo(
        &self,
        negocio_id: &str,
        anio_mes: &str,
    ) -> Result<Vec<Operacion>, DomainError> {
        let mapa = bloquear(&self.operaciones)?;
        Ok(mapa
            .get(negocio_id)
            .map(|lista| {
                lista
                    .iter()
                    .filter(|o| o.fecha.format("%Y-%m").to_string() == anio_mes)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default())
    }
}

#[async_trait]
impl FacturaRepository for AlmacenEnMemoria {
    async fn guardar_factura(
        &self,
        negocio_id: &str,
        factura: Factura,
    ) -> Result<Factura, DomainError> {
        let mut mapa = bloquear(&self.facturas)?;
        let lista = mapa.entry(negocio_id.to_string()).or_default();
        match lista.iter_mut().find(|f| f.id == factura.id) {
            Some(existente) => *existente = factura.clone(),
            None => lista.push(factura.clone()),
        }
        Ok(factura)
    }

    async fn obtener_factura(
        &self,
        negocio_id: &str,
        id: &str,
    ) -> Result<Option<Factura>, DomainError> {
        let mapa = bloquear(&self.facturas)?;
        Ok(mapa
            .get(negocio_id)
            .and_then(|lista| lista.iter().find(|f| f.id == id).cloned()))
    }
}

/// Simulador de Hacienda que acepta siempre.
pub struct HaciendaSimuladorAcepta;

#[async_trait]
impl HaciendaSimulatorPort for HaciendaSimuladorAcepta {
    async fn validar_documento(
        &self,
        factura: &Factura,
    ) -> Result<Result<String, String>, DomainError> {
        Ok(Ok(format!("SIM-{}", factura.numero_control_simulado)))
    }
}

/// Simulador de Hacienda que rechaza los documentos.
pub struct HaciendaSimuladorRechaza;

#[async_trait]
impl HaciendaSimulatorPort for HaciendaSimuladorRechaza {
    async fn validar_documento(
        &self,
        _factura: &Factura,
    ) -> Result<Result<String, String>, DomainError> {
        Ok(Err("Documento rechazado por el simulador".into()))
    }
}

/// Simulador de Hacienda que no puede responder.
pub struct HaciendaSimuladorFalla;

#[async_trait]
impl HaciendaSimulatorPort for HaciendaSimuladorFalla {
    async fn validar_documento(
        &self,
        _factura: &Factura,
    ) -> Result<Result<String, String>, DomainError> {
        Err(DomainError::ValidacionError(
            "El simulador no respondió".into(),
        ))
    }
}

/// Calculadora determinista para pruebas de casos de uso.
pub struct CalculadoraFiscalEnMemoria;

impl CalculadoraFiscalEnMemoria {
    pub fn nuevo() -> Self {
        Self
    }
}

fn regla_vigente<'a>(
    reglas: &'a [ReglaFiscal],
    codigo: &str,
    fecha: NaiveDate,
) -> Result<&'a ReglaFiscal, DomainError> {
    reglas
        .iter()
        .filter(|regla| regla.codigo == codigo)
        .filter(|regla| regla.vigencia_desde <= fecha)
        .filter(|regla| regla.vigencia_hasta.is_none_or(|hasta| fecha <= hasta))
        .filter(|regla| {
            regla
                .parametros
                .get("estado")
                .and_then(serde_json::Value::as_str)
                != Some("por_definir")
        })
        .max_by_key(|regla| regla.version)
        .ok_or_else(|| DomainError::ReglaFiscalNoEncontrada(codigo.into()))
}

fn decimal_regla(regla: &ReglaFiscal, clave: &str) -> Result<Decimal, DomainError> {
    let texto = regla
        .parametros
        .get(clave)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            DomainError::ValidacionError(format!(
                "La regla '{}' no contiene el Decimal '{}'",
                regla.codigo, clave
            ))
        })?;
    texto.parse().map_err(|_| {
        DomainError::ValidacionError(format!(
            "El valor '{}.{}' no es un Decimal válido",
            regla.codigo, clave
        ))
    })
}

fn entero_regla(regla: &ReglaFiscal, clave: &str) -> Result<u32, DomainError> {
    let valor = regla.parametros.get(clave).ok_or_else(|| {
        DomainError::ValidacionError(format!(
            "La regla '{}' no contiene el entero '{}'",
            regla.codigo, clave
        ))
    })?;
    if let Some(numero) = valor.as_u64() {
        return u32::try_from(numero).map_err(|_| {
            DomainError::ValidacionError(format!(
                "El parámetro '{}.{}' excede el rango",
                regla.codigo, clave
            ))
        });
    }
    valor
        .as_str()
        .ok_or_else(|| {
            DomainError::ValidacionError(format!(
                "El parámetro '{}.{}' debe ser un entero",
                regla.codigo, clave
            ))
        })?
        .parse()
        .map_err(|_| {
            DomainError::ValidacionError(format!(
                "El parámetro '{}.{}' no es un entero válido",
                regla.codigo, clave
            ))
        })
}

fn redondear(valor: Decimal) -> Decimal {
    valor.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
}

#[async_trait]
impl CalculadoraFiscalPort for CalculadoraFiscalEnMemoria {
    async fn tasa_iva_vigente(
        &self,
        reglas: &[ReglaFiscal],
        fecha: NaiveDate,
    ) -> Result<Decimal, DomainError> {
        decimal_regla(regla_vigente(reglas, "iva_general", fecha)?, "tasa")
    }

    async fn calcular_resumen(
        &self,
        periodo: Periodo,
        operaciones: &[Operacion],
        reglas: &[ReglaFiscal],
        remanente_periodo_anterior: Option<Decimal>,
    ) -> Result<ResumenImpuestos, DomainError> {
        let (_, fecha_referencia) = periodo.rango_fechas()?;
        let _tasa_iva = decimal_regla(
            regla_vigente(reglas, "iva_general", fecha_referencia)?,
            "tasa",
        )?;
        let tasa_pago = decimal_regla(
            regla_vigente(reglas, "pago_cuenta", fecha_referencia)?,
            "tasa",
        )?;
        let (inicio, fin) = periodo.rango_fechas()?;
        let mut ingresos_gravados = Decimal::ZERO;
        let mut debito = Decimal::ZERO;
        let mut credito = Decimal::ZERO;

        for operacion in operaciones
            .iter()
            .filter(|op| op.fecha >= inicio && op.fecha <= fin)
        {
            match operacion.tipo {
                TipoOperacion::Venta => {
                    ingresos_gravados += operacion.base;
                    debito += operacion.iva;
                }
                TipoOperacion::Compra => credito += operacion.iva,
            }
        }

        let ingresos_gravados = redondear(ingresos_gravados);
        let debito = redondear(debito);
        let credito = redondear(credito);
        let pago_a_cuenta = redondear(ingresos_gravados * tasa_pago);
        let neto =
            redondear(debito - credito - remanente_periodo_anterior.unwrap_or(Decimal::ZERO));
        let (iva_a_pagar, remanente) = if neto > Decimal::ZERO {
            (neto, Decimal::ZERO)
        } else {
            (Decimal::ZERO, -neto)
        };

        Ok(ResumenImpuestos {
            periodo,
            ingresos_gravados,
            debito,
            credito,
            iva_a_pagar,
            remanente,
            remanente_periodo_anterior: remanente_periodo_anterior.map(redondear),
            pago_a_cuenta,
        })
    }

    async fn generar_obligaciones(
        &self,
        periodo: Periodo,
        reglas: &[ReglaFiscal],
        feriados: &[NaiveDate],
    ) -> Result<Vec<ObligacionFiscal>, DomainError> {
        let (_, fecha_referencia) = periodo.rango_fechas()?;
        let plazos = [
            (TipoObligacion::IvaF07, "plazo_iva_f07"),
            (TipoObligacion::PagoCuentaF14, "plazo_pago_cuenta_f14"),
            (TipoObligacion::RentaF11, "plazo_renta_anual_f11"),
        ];
        plazos
            .into_iter()
            .map(|(tipo, codigo)| {
                let regla = regla_vigente(reglas, codigo, fecha_referencia)?;
                let modo = regla
                    .parametros
                    .get("modo")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| {
                        DomainError::ValidacionError(format!(
                            "La regla '{}' no define el modo de vencimiento",
                            regla.codigo
                        ))
                    })?;
                let fecha_limite = match modo {
                    "dias_habiles_mes_siguiente" => {
                        let dias = entero_regla(regla, "dias")?;
                        if dias == 0 {
                            return Err(DomainError::ValidacionError(
                                "El número de días hábiles debe ser mayor que cero".into(),
                            ));
                        }
                        let siguiente = periodo.siguiente();
                        let mut fecha =
                            NaiveDate::from_ymd_opt(siguiente.anio(), siguiente.mes(), 1)
                                .ok_or_else(|| DomainError::FechaInvalida(siguiente.to_string()))?;
                        let mut contados = 0;
                        while contados < dias {
                            if !matches!(fecha.weekday(), Weekday::Sat | Weekday::Sun)
                                && !feriados.contains(&fecha)
                            {
                                contados += 1;
                            }
                            if contados < dias {
                                fecha = fecha
                                    .succ_opt()
                                    .ok_or_else(|| DomainError::FechaInvalida(fecha.to_string()))?;
                            }
                        }
                        fecha
                    }
                    "fecha_fija_anual" => NaiveDate::from_ymd_opt(
                        periodo.anio(),
                        entero_regla(regla, "mes")?,
                        entero_regla(regla, "dia")?,
                    )
                    .ok_or_else(|| {
                        DomainError::FechaInvalida(format!("{}.mes/dia", regla.codigo))
                    })?,
                    otro => {
                        return Err(DomainError::ValidacionError(format!(
                            "Modo de vencimiento no soportado: '{otro}'"
                        )))
                    }
                };
                Ok(ObligacionFiscal { tipo, fecha_limite })
            })
            .collect()
    }
}

#[async_trait]
impl ReglaFiscalRepository for AlmacenEnMemoria {
    async fn obtener_regla(
        &self,
        codigo: &str,
        fecha: NaiveDate,
    ) -> Result<Option<ReglaFiscal>, DomainError> {
        let reglas = bloquear(&self.reglas)?;
        Ok(reglas
            .iter()
            .find(|r| {
                let vigente = match r.vigencia_hasta {
                    Some(hasta) => fecha <= hasta,
                    None => true,
                };
                r.codigo == codigo && r.vigencia_desde <= fecha && vigente
            })
            .cloned())
    }

    async fn guardar_regla(&self, regla: ReglaFiscal) -> Result<ReglaFiscal, DomainError> {
        let mut reglas = bloquear(&self.reglas)?;
        match reglas.iter_mut().find(|r| r.id == regla.id) {
            Some(existente) => *existente = regla.clone(),
            None => reglas.push(regla.clone()),
        }
        Ok(regla)
    }
}

#[async_trait]
impl DeclaracionRepository for AlmacenEnMemoria {
    async fn guardar_declaracion(
        &self,
        negocio_id: &str,
        declaracion: Declaracion,
    ) -> Result<Declaracion, DomainError> {
        let mut mapa = bloquear(&self.declaraciones)?;
        let lista = mapa.entry(negocio_id.to_string()).or_default();
        match lista.iter_mut().find(|d| {
            d.periodo == declaracion.periodo && d.tipo_impuesto == declaracion.tipo_impuesto
        }) {
            Some(existente) => *existente = declaracion.clone(),
            None => lista.push(declaracion.clone()),
        }
        Ok(declaracion)
    }

    async fn obtener_declaracion(
        &self,
        negocio_id: &str,
        periodo: &str,
        tipo_impuesto: TipoImpuesto,
    ) -> Result<Option<Declaracion>, DomainError> {
        let mapa = bloquear(&self.declaraciones)?;
        Ok(mapa.get(negocio_id).and_then(|lista| {
            lista
                .iter()
                .find(|d| d.periodo == periodo && d.tipo_impuesto == tipo_impuesto)
                .cloned()
        }))
    }
}

#[async_trait]
impl RecordatorioRepository for AlmacenEnMemoria {
    async fn listar_proximos(&self, negocio_id: &str) -> Result<Vec<Recordatorio>, DomainError> {
        let mut recordatorios: Vec<Recordatorio> = bloquear(&self.recordatorios)?
            .iter()
            .filter(|r| r.negocio_id == negocio_id)
            .cloned()
            .collect();
        recordatorios.sort_by_key(|r| r.fecha_limite);
        Ok(recordatorios)
    }

    async fn guardar_recordatorio(
        &self,
        recordatorio: Recordatorio,
    ) -> Result<Recordatorio, DomainError> {
        let mut recordatorios = bloquear(&self.recordatorios)?;
        match recordatorios.iter_mut().find(|r| r.id == recordatorio.id) {
            Some(existente) => *existente = recordatorio.clone(),
            None => recordatorios.push(recordatorio.clone()),
        }
        Ok(recordatorio)
    }
}

#[async_trait]
impl SyncStatusPort for AlmacenEnMemoria {
    async fn obtener_estado(&self, negocio_id: &str) -> Result<serde_json::Value, DomainError> {
        let pendientes = bloquear(&self.cola)?.pendientes.len();
        let conflictos_pendientes = bloquear(&self.conflictos)?
            .iter()
            .filter(|c| c.estado == EstadoConflicto::Pendiente)
            .count();
        Ok(json!({
            "negocio_id": negocio_id,
            "pendientes": pendientes,
            "conflictos_pendientes": conflictos_pendientes,
        }))
    }
}

#[async_trait]
impl ConflictoRepository for AlmacenEnMemoria {
    async fn guardar_conflicto(&self, conflicto: Conflicto) -> Result<Conflicto, DomainError> {
        let mut conflictos = bloquear(&self.conflictos)?;
        match conflictos.iter_mut().find(|c| c.doc_id == conflicto.doc_id) {
            Some(existente) => *existente = conflicto.clone(),
            None => conflictos.push(conflicto.clone()),
        }
        Ok(conflicto)
    }

    async fn listar_pendientes(&self) -> Result<Vec<Conflicto>, DomainError> {
        Ok(bloquear(&self.conflictos)?
            .iter()
            .filter(|c| c.estado == EstadoConflicto::Pendiente)
            .cloned()
            .collect())
    }
}

#[async_trait]
impl NotificacionPort for AlmacenEnMemoria {
    async fn notificar_recordatorio(&self, recordatorio: &Recordatorio) -> Result<(), DomainError> {
        bloquear(&self.notificaciones)?.push(recordatorio.clone());
        Ok(())
    }
}

#[async_trait]
impl ColaSyncPort for AlmacenEnMemoria {
    async fn encolar(&self, item: ItemSync) -> Result<(), DomainError> {
        let mut cola = bloquear(&self.cola)?;
        if cola.aplicadas.contains(&item.id) {
            return Ok(());
        }
        match cola.pendientes.iter_mut().find(|i| i.id == item.id) {
            Some(existente) => *existente = item,
            None => cola.pendientes.push(item),
        }
        Ok(())
    }

    async fn pendientes(&self) -> Result<Vec<ItemSync>, DomainError> {
        Ok(bloquear(&self.cola)?.pendientes.clone())
    }

    async fn marcar_aplicada(&self, id_cliente: &str) -> Result<(), DomainError> {
        let mut cola = bloquear(&self.cola)?;
        cola.pendientes.retain(|i| i.id != id_cliente);
        cola.aplicadas.insert(id_cliente.to_string());
        Ok(())
    }
}

/// Aplicador idempotente configurable para pruebas de reenvío, fallos y conflictos.
#[derive(Default)]
pub struct AplicadorSyncEnMemoria {
    aplicados: Mutex<HashSet<String>>,
    fallos_restantes: Mutex<HashMap<String, u32>>,
    intentos: Mutex<HashMap<String, u32>>,
    conflictos: Mutex<HashSet<String>>,
}

impl AplicadorSyncEnMemoria {
    pub fn fallar_siguientes_intentos(
        &self,
        id: impl Into<String>,
        cantidad: u32,
    ) -> Result<(), DomainError> {
        bloquear(&self.fallos_restantes)?.insert(id.into(), cantidad);
        Ok(())
    }

    pub fn marcar_como_conflicto(&self, id: impl Into<String>) -> Result<(), DomainError> {
        bloquear(&self.conflictos)?.insert(id.into());
        Ok(())
    }

    pub fn cantidad_aplicaciones(&self, id: &str) -> Result<u32, DomainError> {
        Ok(u32::from(bloquear(&self.aplicados)?.contains(id)))
    }

    pub fn intentos(&self, id: &str) -> Result<u32, DomainError> {
        Ok(bloquear(&self.intentos)?
            .get(id)
            .copied()
            .unwrap_or_default())
    }
}

#[async_trait]
impl AplicadorSyncPort for AplicadorSyncEnMemoria {
    async fn aplicar_idempotente(
        &self,
        item: &ItemSync,
    ) -> Result<ResultadoAplicacionSync, DomainError> {
        {
            let mut intentos = bloquear(&self.intentos)?;
            let cantidad = intentos.entry(item.id.clone()).or_default();
            *cantidad += 1;
        }

        {
            let mut fallos = bloquear(&self.fallos_restantes)?;
            if let Some(restantes) = fallos.get_mut(&item.id) {
                if *restantes > 0 {
                    *restantes -= 1;
                    return Err(DomainError::ValidacionError(
                        "Fallo transitorio de sincronización".into(),
                    ));
                }
            }
        }

        if !bloquear(&self.aplicados)?.insert(item.id.clone()) {
            return Ok(ResultadoAplicacionSync::YaAplicada);
        }
        if bloquear(&self.conflictos)?.contains(&item.id) {
            Ok(ResultadoAplicacionSync::Conflicto)
        } else {
            Ok(ResultadoAplicacionSync::Aplicada)
        }
    }
}

/// Espera en memoria que registra las duraciones sin suspender la prueba.
#[derive(Default)]
pub struct EsperaSyncEnMemoria {
    duraciones: Mutex<Vec<std::time::Duration>>,
}

impl EsperaSyncEnMemoria {
    pub fn duraciones(&self) -> Result<Vec<std::time::Duration>, DomainError> {
        Ok(bloquear(&self.duraciones)?.clone())
    }
}

#[async_trait]
impl EsperaSyncPort for EsperaSyncEnMemoria {
    async fn esperar(&self, duracion: std::time::Duration) -> Result<(), DomainError> {
        bloquear(&self.duraciones)?.push(duracion);
        Ok(())
    }
}

/// Reloj fijo, para pruebas deterministas.
#[derive(Debug, Clone)]
pub struct RelojFijo {
    momento: DateTime<Utc>,
}

impl RelojFijo {
    pub fn new(momento: DateTime<Utc>) -> Self {
        Self { momento }
    }
}

impl Default for RelojFijo {
    fn default() -> Self {
        Self {
            momento: DateTime::<Utc>::from_timestamp(0, 0).unwrap_or_else(Utc::now),
        }
    }
}

impl RelojPort for RelojFijo {
    fn ahora_utc(&self) -> DateTime<Utc> {
        self.momento
    }

    fn hoy(&self) -> NaiveDate {
        self.momento.date_naive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::TipoOperacion;
    use rust_decimal_macros::dec;

    fn operacion(id: &str) -> Operacion {
        Operacion {
            id: id.into(),
            rev: None,
            tipo: TipoOperacion::Venta,
            fecha: NaiveDate::from_ymd_opt(2026, 11, 5).unwrap(),
            descripcion: "Venta de prueba".into(),
            base: dec!(100.00),
            iva: dec!(13.00),
            total: dec!(113.00),
            contraparte: "Cliente A".into(),
            creado_por: "usr1".into(),
            origen: "local".into(),
            creado_en: Utc::now(),
        }
    }

    #[tokio::test]
    async fn guardar_y_listar_operaciones_en_memoria() {
        let almacen = AlmacenEnMemoria::nuevo();

        almacen
            .guardar_operacion("negocio_001", operacion("op-1"))
            .await
            .unwrap();
        almacen
            .guardar_operacion("negocio_001", operacion("op-2"))
            .await
            .unwrap();
        almacen
            .guardar_operacion("negocio_002", operacion("op-3"))
            .await
            .unwrap();

        let listado = almacen
            .listar_por_periodo("negocio_001", "2026-11")
            .await
            .unwrap();
        assert_eq!(listado.len(), 2);

        let obtenida = almacen
            .obtener_operacion("negocio_001", "op-1")
            .await
            .unwrap();
        assert_eq!(obtenida.map(|o| o.id), Some("op-1".to_string()));

        let otro_negocio = almacen
            .listar_por_periodo("negocio_002", "2026-11")
            .await
            .unwrap();
        assert_eq!(otro_negocio.len(), 1);
    }

    #[tokio::test]
    async fn cola_sync_es_idempotente_por_uuid_del_cliente() {
        let almacen = AlmacenEnMemoria::nuevo();
        let item = ItemSync {
            id: "uuid-cliente-1".into(),
            endpoint: "/ventas".into(),
            payload: json!({"base": "100.00"}),
            timestamp: Utc::now(),
        };

        almacen.encolar(item.clone()).await.unwrap();
        almacen.encolar(item.clone()).await.unwrap();
        assert_eq!(almacen.pendientes().await.unwrap().len(), 1);

        almacen.marcar_aplicada("uuid-cliente-1").await.unwrap();
        almacen.marcar_aplicada("uuid-cliente-1").await.unwrap();
        assert!(almacen.pendientes().await.unwrap().is_empty());

        // Un elemento ya aplicado no se reencola.
        almacen.encolar(item).await.unwrap();
        assert!(almacen.pendientes().await.unwrap().is_empty());
    }
}
