use crate::ports::*;
use domain::*;
use std::sync::Arc;

pub struct GestionOperacionesUseCase {
    repo: Arc<dyn OperacionRepository>,
}

impl GestionOperacionesUseCase {
    pub fn new(repo: Arc<dyn OperacionRepository>) -> Self {
        Self { repo }
    }

    pub async fn registrar(&self, negocio_id: &str, operacion: Operacion) -> Result<Operacion, DomainError> {
        // Validación básica
        if operacion.base < rust_decimal::Decimal::ZERO || operacion.iva < rust_decimal::Decimal::ZERO {
            return Err(DomainError::ImporteInvalido);
        }
        self.repo.guardar_operacion(negocio_id, operacion).await
    }

    pub async fn listar_periodo(&self, negocio_id: &str, anio_mes: &str) -> Result<Vec<Operacion>, DomainError> {
        self.repo.listar_por_periodo(negocio_id, anio_mes).await
    }
}
