use std::sync::Arc;

use chrono::NaiveDate;
use domain::{DomainError, EstadoRecordatorio, Periodo, Recordatorio, TipoObligacion};

use super::{
    cargar_reglas, CODIGO_PLAZO_IVA_F07, CODIGO_PLAZO_PAGO_CUENTA_F14, CODIGO_PLAZO_RENTA_ANUAL_F11,
};
use crate::ports::{CalculadoraFiscalPort, RecordatorioRepository, ReglaFiscalRepository};

/// Genera y persiste los recordatorios pendientes de un período.
pub struct ListarRecordatorios {
    reglas: Arc<dyn ReglaFiscalRepository>,
    recordatorios: Arc<dyn RecordatorioRepository>,
    calculadora: Arc<dyn CalculadoraFiscalPort>,
}

impl ListarRecordatorios {
    pub fn new(
        reglas: Arc<dyn ReglaFiscalRepository>,
        recordatorios: Arc<dyn RecordatorioRepository>,
        calculadora: Arc<dyn CalculadoraFiscalPort>,
    ) -> Self {
        Self {
            reglas,
            recordatorios,
            calculadora,
        }
    }

    pub async fn ejecutar(
        &self,
        negocio_id: &str,
        periodo: Periodo,
        feriados: &[NaiveDate],
    ) -> Result<Vec<Recordatorio>, DomainError> {
        let (_, fecha_referencia) = periodo.rango_fechas()?;
        let reglas = cargar_reglas(
            self.reglas.as_ref(),
            &[
                CODIGO_PLAZO_IVA_F07,
                CODIGO_PLAZO_PAGO_CUENTA_F14,
                CODIGO_PLAZO_RENTA_ANUAL_F11,
            ],
            fecha_referencia,
        )
        .await?;
        let obligaciones = self
            .calculadora
            .generar_obligaciones(periodo, &reglas, feriados)
            .await?;

        let mut guardados = Vec::with_capacity(obligaciones.len());
        for obligacion in obligaciones {
            let recordatorio = Recordatorio {
                id: format!(
                    "recordatorio:{negocio_id}:{periodo}:{}",
                    slug_obligacion(obligacion.tipo)
                ),
                rev: None,
                tipo: "recordatorio".into(),
                obligacion: obligacion.tipo,
                fecha_limite: obligacion.fecha_limite,
                estado: EstadoRecordatorio::Pendiente,
                negocio_id: negocio_id.to_string(),
            };
            guardados.push(
                self.recordatorios
                    .guardar_recordatorio(recordatorio)
                    .await?,
            );
        }

        Ok(guardados)
    }
}

fn slug_obligacion(tipo: TipoObligacion) -> &'static str {
    match tipo {
        TipoObligacion::IvaF07 => "iva_f07",
        TipoObligacion::PagoCuentaF14 => "pago_cuenta_f14",
        TipoObligacion::RentaF11 => "renta_f11",
    }
}
