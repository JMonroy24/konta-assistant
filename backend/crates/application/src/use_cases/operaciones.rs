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

/// Consulta operaciones de un período (AAAA-MM) para un negocio específico.
pub struct ConsultarOperaciones {
    operaciones: Arc<dyn OperacionRepository>,
}

impl ConsultarOperaciones {
    pub fn new(operaciones: Arc<dyn OperacionRepository>) -> Self {
        Self { operaciones }
    }

    pub async fn ejecutar(
        &self,
        negocio_id: &str,
        anio_mes: &str,
    ) -> Result<Vec<Operacion>, DomainError> {
        if negocio_id.trim().is_empty() {
            return Err(DomainError::ValidacionError(
                "El identificador de negocio no puede estar vacío".into(),
            ));
        }

        // Valida que el período cumpla estrictamente con el formato YYYY-MM
        let periodo = domain::Periodo::parse(anio_mes)?;
        self.operaciones
            .listar_por_periodo(negocio_id, &periodo.to_string())
            .await
    }
}
