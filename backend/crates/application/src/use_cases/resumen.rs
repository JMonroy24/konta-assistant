use std::sync::Arc;

use domain::{DomainError, Periodo, ResumenImpuestos};
use rust_decimal::Decimal;

use super::{cargar_reglas, CODIGO_IVA_GENERAL, CODIGO_PAGO_CUENTA};
use crate::ports::{CalculadoraFiscalPort, OperacionRepository, ReglaFiscalRepository};

/// Calcula el resumen de IVA y pago a cuenta de un período.
pub struct ConsultarResumenImpuestos {
    operaciones: Arc<dyn OperacionRepository>,
    reglas: Arc<dyn ReglaFiscalRepository>,
    calculadora: Arc<dyn CalculadoraFiscalPort>,
}

impl ConsultarResumenImpuestos {
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
        periodo: Periodo,
        remanente_periodo_anterior: Option<Decimal>,
    ) -> Result<ResumenImpuestos, DomainError> {
        let (_, fecha_referencia) = periodo.rango_fechas()?;
        let operaciones = self
            .operaciones
            .listar_por_periodo(negocio_id, &periodo.to_string())
            .await?;
        let reglas = cargar_reglas(
            self.reglas.as_ref(),
            &[CODIGO_IVA_GENERAL, CODIGO_PAGO_CUENTA],
            fecha_referencia,
        )
        .await?;

        self.calculadora
            .calcular_resumen(periodo, &operaciones, &reglas, remanente_periodo_anterior)
            .await
    }
}
