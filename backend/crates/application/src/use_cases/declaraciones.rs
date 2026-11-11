use std::sync::Arc;

use domain::{Declaracion, DomainError, EstadoDeclaracion, Periodo, TipoImpuesto};

use super::{cargar_reglas, CODIGO_IVA_GENERAL, CODIGO_PAGO_CUENTA};
use crate::ports::{
    CalculadoraFiscalPort, DeclaracionRepository, OperacionRepository, ReglaFiscalRepository,
};

/// Genera el borrador de la declaración de un período.
///
/// Es idempotente por período y tipo de impuesto: si ya existe un borrador,
/// devuelve el existente sin recalcular.
pub struct GenerarBorradorDeclaracion {
    operaciones: Arc<dyn OperacionRepository>,
    reglas: Arc<dyn ReglaFiscalRepository>,
    declaraciones: Arc<dyn DeclaracionRepository>,
    calculadora: Arc<dyn CalculadoraFiscalPort>,
}

impl GenerarBorradorDeclaracion {
    pub fn new(
        operaciones: Arc<dyn OperacionRepository>,
        reglas: Arc<dyn ReglaFiscalRepository>,
        declaraciones: Arc<dyn DeclaracionRepository>,
        calculadora: Arc<dyn CalculadoraFiscalPort>,
    ) -> Self {
        Self {
            operaciones,
            reglas,
            declaraciones,
            calculadora,
        }
    }

    pub async fn ejecutar(
        &self,
        negocio_id: &str,
        periodo: Periodo,
        tipo_impuesto: TipoImpuesto,
    ) -> Result<Declaracion, DomainError> {
        let periodo_texto = periodo.to_string();

        if let Some(existente) = self
            .declaraciones
            .obtener_declaracion(negocio_id, &periodo_texto, tipo_impuesto)
            .await?
        {
            return Ok(existente);
        }

        let (_, fecha_referencia) = periodo.rango_fechas()?;
        let operaciones = self
            .operaciones
            .listar_por_periodo(negocio_id, &periodo_texto)
            .await?;
        let reglas = cargar_reglas(
            self.reglas.as_ref(),
            &[CODIGO_IVA_GENERAL, CODIGO_PAGO_CUENTA],
            fecha_referencia,
        )
        .await?;
        let resumen = self
            .calculadora
            .calcular_resumen(periodo, &operaciones, &reglas, None)
            .await?;

        let montos_calculados = serde_json::to_value(&resumen).map_err(|error| {
            DomainError::ValidacionError(format!("No se pudo serializar el resumen: {error}"))
        })?;

        let declaracion = Declaracion {
            id: format!(
                "declaracion:{negocio_id}:{periodo_texto}:{}",
                slug_tipo(tipo_impuesto)
            ),
            rev: None,
            tipo: "declaracion".into(),
            periodo: periodo_texto,
            tipo_impuesto,
            montos_calculados,
            estado: EstadoDeclaracion::Borrador,
            fecha_presentacion: None,
        };

        self.declaraciones
            .guardar_declaracion(negocio_id, declaracion)
            .await
    }
}

fn slug_tipo(tipo: TipoImpuesto) -> &'static str {
    match tipo {
        TipoImpuesto::IvaF07 => "iva_f07",
        TipoImpuesto::PagoCuentaF14 => "pago_cuenta_f14",
        TipoImpuesto::RentaF11 => "renta_f11",
    }
}
