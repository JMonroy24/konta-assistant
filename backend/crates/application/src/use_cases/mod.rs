//! Casos de uso de la aplicación.
//!
//! Todos reciben sus dependencias como `Arc<dyn Puerto>`, de modo que la
//! infraestructura real y los dobles de prueba son intercambiables.

mod declaraciones;
mod facturas;
mod operaciones;
mod recordatorios;
mod resumen;
mod sincronizacion;

pub use declaraciones::GenerarBorradorDeclaracion;
pub use facturas::EmitirFactura;
pub use operaciones::RegistrarOperacion;
pub use recordatorios::ListarRecordatorios;
pub use resumen::ConsultarResumenImpuestos;
pub use sincronizacion::{
    ContadoresSync, DetectarConflictos, ResumenContadoresSync, SincronizarCola,
};

use chrono::NaiveDate;
use domain::{DomainError, ReglaFiscal};

use crate::ports::ReglaFiscalRepository;

/// Códigos de regla que consumen los casos de uso.
pub const CODIGO_IVA_GENERAL: &str = "iva_general";
pub const CODIGO_PAGO_CUENTA: &str = "pago_cuenta";
pub const CODIGO_PLAZO_IVA_F07: &str = "plazo_iva_f07";
pub const CODIGO_PLAZO_PAGO_CUENTA_F14: &str = "plazo_pago_cuenta_f14";
pub const CODIGO_PLAZO_RENTA_ANUAL_F11: &str = "plazo_renta_anual_f11";

/// Carga las reglas indicadas vigentes en `fecha`, omitiendo las ausentes.
pub(crate) async fn cargar_reglas(
    repo: &dyn ReglaFiscalRepository,
    codigos: &[&str],
    fecha: NaiveDate,
) -> Result<Vec<ReglaFiscal>, DomainError> {
    let mut reglas = Vec::with_capacity(codigos.len());
    for codigo in codigos {
        if let Some(regla) = repo.obtener_regla(codigo, fecha).await? {
            reglas.push(regla);
        }
    }
    Ok(reglas)
}

#[cfg(all(test, feature = "testing"))]
mod tests;
