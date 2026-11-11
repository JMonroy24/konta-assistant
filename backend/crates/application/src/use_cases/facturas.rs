use std::sync::Arc;

use domain::{DomainError, EstadoFactura, Factura};

use crate::ports::{FacturaRepository, HaciendaSimulatorPort, RelojPort};

/// Emite una factura y refleja la respuesta del simulador de Hacienda.
///
/// La factura se persiste primero como `Pendiente`; después se envía al
/// simulador. Nunca queda aceptada sin respuesta: si el simulador falla, la
/// factura permanece `Pendiente` y se propaga el error.
pub struct EmitirFactura {
    facturas: Arc<dyn FacturaRepository>,
    simulador: Arc<dyn HaciendaSimulatorPort>,
    reloj: Arc<dyn RelojPort>,
}

impl EmitirFactura {
    pub fn new(
        facturas: Arc<dyn FacturaRepository>,
        simulador: Arc<dyn HaciendaSimulatorPort>,
        reloj: Arc<dyn RelojPort>,
    ) -> Self {
        Self {
            facturas,
            simulador,
            reloj,
        }
    }

    pub async fn ejecutar(
        &self,
        negocio_id: &str,
        referencia_venta_id: &str,
        numero_control_simulado: &str,
    ) -> Result<Factura, DomainError> {
        let mut factura = Factura {
            id: format!("factura:{negocio_id}:{referencia_venta_id}"),
            rev: None,
            tipo: "factura".into(),
            numero_control_simulado: numero_control_simulado.to_string(),
            estado_validacion: EstadoFactura::Pendiente,
            referencia_venta_id: referencia_venta_id.to_string(),
            hora_emision: self.reloj.ahora_utc(),
            mensaje_simulador: None,
        };

        self.facturas
            .guardar_factura(negocio_id, factura.clone())
            .await?;

        match self.simulador.validar_documento(&factura).await {
            Ok(Ok(mensaje)) => {
                factura.estado_validacion = EstadoFactura::AceptadoSimulado;
                factura.mensaje_simulador = Some(mensaje);
            }
            Ok(Err(mensaje)) => {
                factura.estado_validacion = EstadoFactura::Rechazado;
                factura.mensaje_simulador = Some(mensaje);
            }
            Err(error) => return Err(error),
        }

        self.facturas.guardar_factura(negocio_id, factura).await
    }
}
