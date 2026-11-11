#![allow(clippy::double_must_use)]

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use domain::{
    Conflicto, Declaracion, DomainError, Factura, ObligacionFiscal, Operacion, Periodo,
    Recordatorio, ReglaFiscal, ResumenImpuestos, TipoImpuesto,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait OperacionRepository: Send + Sync {
    async fn guardar_operacion(
        &self,
        negocio_id: &str,
        operacion: Operacion,
    ) -> Result<Operacion, DomainError>;
    async fn obtener_operacion(
        &self,
        negocio_id: &str,
        id: &str,
    ) -> Result<Option<Operacion>, DomainError>;
    async fn listar_por_periodo(
        &self,
        negocio_id: &str,
        anio_mes: &str,
    ) -> Result<Vec<Operacion>, DomainError>;
}

#[async_trait]
pub trait FacturaRepository: Send + Sync {
    async fn guardar_factura(
        &self,
        negocio_id: &str,
        factura: Factura,
    ) -> Result<Factura, DomainError>;
    async fn obtener_factura(
        &self,
        negocio_id: &str,
        id: &str,
    ) -> Result<Option<Factura>, DomainError>;
}

#[async_trait]
pub trait HaciendaSimulatorPort: Send + Sync {
    async fn validar_documento(
        &self,
        factura: &Factura,
    ) -> Result<Result<String, String>, DomainError>;
}

#[async_trait]
pub trait ReglaFiscalRepository: Send + Sync {
    async fn obtener_regla(
        &self,
        codigo: &str,
        fecha: NaiveDate,
    ) -> Result<Option<ReglaFiscal>, DomainError>;
    async fn guardar_regla(&self, regla: ReglaFiscal) -> Result<ReglaFiscal, DomainError>;
}

/// Motor de cálculo fiscal. Se inyecta como puerto para que `application` no
/// dependa de `tax-engine`; la implementación de producción vive en ese crate.
#[async_trait]
pub trait CalculadoraFiscalPort: Send + Sync {
    /// Tasa de IVA vigente en `fecha` según las reglas recibidas.
    async fn tasa_iva_vigente(
        &self,
        reglas: &[ReglaFiscal],
        fecha: NaiveDate,
    ) -> Result<Decimal, DomainError>;

    /// Resumen de IVA y pago a cuenta de un período.
    async fn calcular_resumen(
        &self,
        periodo: Periodo,
        operaciones: &[Operacion],
        reglas: &[ReglaFiscal],
        remanente_periodo_anterior: Option<Decimal>,
    ) -> Result<ResumenImpuestos, DomainError>;

    /// Obligaciones fiscales del período con sus fechas límite.
    async fn generar_obligaciones(
        &self,
        periodo: Periodo,
        reglas: &[ReglaFiscal],
        feriados: &[NaiveDate],
    ) -> Result<Vec<ObligacionFiscal>, DomainError>;
}

#[async_trait]
pub trait DeclaracionRepository: Send + Sync {
    async fn guardar_declaracion(
        &self,
        negocio_id: &str,
        declaracion: Declaracion,
    ) -> Result<Declaracion, DomainError>;
    async fn obtener_declaracion(
        &self,
        negocio_id: &str,
        periodo: &str,
        tipo_impuesto: TipoImpuesto,
    ) -> Result<Option<Declaracion>, DomainError>;
}

#[async_trait]
pub trait RecordatorioRepository: Send + Sync {
    async fn listar_proximos(&self, negocio_id: &str) -> Result<Vec<Recordatorio>, DomainError>;
    async fn guardar_recordatorio(
        &self,
        recordatorio: Recordatorio,
    ) -> Result<Recordatorio, DomainError>;
}

#[async_trait]
pub trait SyncStatusPort: Send + Sync {
    async fn obtener_estado(&self, negocio_id: &str) -> Result<serde_json::Value, DomainError>;
}

#[async_trait]
pub trait ConflictoRepository: Send + Sync {
    async fn guardar_conflicto(&self, conflicto: Conflicto) -> Result<Conflicto, DomainError>;
    async fn listar_pendientes(&self) -> Result<Vec<Conflicto>, DomainError>;
}

/// Acceso al tiempo. Se abstrae para poder fijarlo en pruebas.
pub trait RelojPort: Send + Sync {
    fn ahora_utc(&self) -> DateTime<Utc>;
    fn hoy(&self) -> NaiveDate;
}

#[async_trait]
pub trait NotificacionPort: Send + Sync {
    async fn notificar_recordatorio(&self, recordatorio: &Recordatorio) -> Result<(), DomainError>;
}

/// Elemento de la cola de sincronización offline.
///
/// `id` es el UUID generado por el cliente y actúa como clave de idempotencia:
/// reencolar o aplicar dos veces el mismo elemento no produce duplicados.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemSync {
    pub id: String,
    pub endpoint: String,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

#[async_trait]
pub trait ColaSyncPort: Send + Sync {
    async fn encolar(&self, item: ItemSync) -> Result<(), DomainError>;
    async fn pendientes(&self) -> Result<Vec<ItemSync>, DomainError>;
    async fn marcar_aplicada(&self, id_cliente: &str) -> Result<(), DomainError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultadoAplicacionSync {
    Aplicada,
    YaAplicada,
    Conflicto,
}

/// Aplica el elemento de forma atómica e idempotente usando `item.id` como clave.
#[async_trait]
pub trait AplicadorSyncPort: Send + Sync {
    async fn aplicar_idempotente(
        &self,
        item: &ItemSync,
    ) -> Result<ResultadoAplicacionSync, DomainError>;
}

/// Espera entre intentos de sincronización; permite evitar esperas reales en pruebas.
#[async_trait]
pub trait EsperaSyncPort: Send + Sync {
    async fn esperar(&self, duracion: std::time::Duration) -> Result<(), DomainError>;
}
