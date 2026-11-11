use std::sync::Arc;

use domain::{DomainError, Operacion};

use super::{cargar_reglas, CODIGO_IVA_GENERAL};
use crate::ports::{CalculadoraFiscalPort, OperacionRepository, ReglaFiscalRepository};

/// Registra una operación tras validarla contra la tasa de IVA vigente.
pub struct RegistrarOperacion {
    operaciones: Arc<dyn OperacionRepository>,
    reglas: Arc<dyn ReglaFiscalRepository>,
    calculadora: Arc<dyn CalculadoraFiscalPort>,
}

impl RegistrarOperacion {
    pub fn new(
        operaciones: Arc<dyn OperacionRepository>,
        reglas: Arc<dyn ReglaFiscalRepository>,
        calculadora: Arc<dyn CalculadoraFiscalPort>,
    ) -> Self {
        Self {
            operaciones,
            reglas,
            calculadora,
        }
    }

    pub async fn ejecutar(
        &self,
        negocio_id: &str,
        operacion: Operacion,
    ) -> Result<Operacion, DomainError> {
        let reglas =
            cargar_reglas(self.reglas.as_ref(), &[CODIGO_IVA_GENERAL], operacion.fecha).await?;
        let tasa = self
            .calculadora
            .tasa_iva_vigente(&reglas, operacion.fecha)
            .await?;
        operacion.validar(tasa)?;
        self.operaciones
            .guardar_operacion(negocio_id, operacion)
            .await
    }
}
