use async_trait::async_trait;
use domain::{
    Declaracion, DomainError, Factura, Operacion, Recordatorio, ReglaFiscal,
};

#[async_trait]
pub trait OperacionRepository: Send + Sync {
    async fn guardar_operacion(&self, negocio_id: &str, operacion: Operacion) -> Result<Operacion, DomainError>;
    async fn obtener_operacion(&self, negocio_id: &str, id: &str) -> Result<Option<Operacion>, DomainError>;
    async fn listar_por_periodo(&self, negocio_id: &str, anio_mes: &str) -> Result<Vec<Operacion>, DomainError>;
}

#[async_trait]
pub trait FacturaRepository: Send + Sync {
    async fn guardar_factura(&self, negocio_id: &str, factura: Factura) -> Result<Factura, DomainError>;
    async fn obtener_factura(&self, negocio_id: &str, id: &str) -> Result<Option<Factura>, DomainError>;
}

#[async_trait]
pub trait HaciendaSimulatorPort: Send + Sync {
    async fn validar_documento(&self, factura: &Factura) -> Result<Result<String, String>, DomainError>;
}

#[async_trait]
pub trait ReglaFiscalRepository: Send + Sync {
    async fn obtener_regla(&self, codigo: &str, fecha: chrono::NaiveDate) -> Result<Option<ReglaFiscal>, DomainError>;
    async fn guardar_regla(&self, regla: ReglaFiscal) -> Result<ReglaFiscal, DomainError>;
}

#[async_trait]
pub trait DeclaracionRepository: Send + Sync {
    async fn guardar_declaracion(&self, negocio_id: &str, declaracion: Declaracion) -> Result<Declaracion, DomainError>;
    async fn obtener_declaracion(&self, negocio_id: &str, periodo: &str) -> Result<Option<Declaracion>, DomainError>;
}

#[async_trait]
pub trait RecordatorioRepository: Send + Sync {
    async fn listar_proximos(&self, negocio_id: &str) -> Result<Vec<Recordatorio>, DomainError>;
    async fn guardar_recordatorio(&self, recordatorio: Recordatorio) -> Result<Recordatorio, DomainError>;
}

#[async_trait]
pub trait SyncStatusPort: Send + Sync {
    async fn obtener_estado(&self, negocio_id: &str) -> Result<serde_json::Value, DomainError>;
}
