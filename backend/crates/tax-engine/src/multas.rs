//! Estimación de multas por presentación extemporánea.
//!
//! Regla de oro: mientras la regla no esté verificada (o esté marcada como
//! `por_definir`), no se estima nada. Los porcentajes provienen siempre de la
//! regla; nunca se inventan valores.

use domain::{DomainError, ReglaFiscal};
use rust_decimal::Decimal;

use crate::redondear;
use crate::reglas::{decimal_desde, esta_por_definir};

/// Multa estimada para una presentación extemporánea.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultaEstimada {
    pub monto_multa: Decimal,
    pub dias_atraso: u32,
}

/// Estima la multa por presentar una declaración `dias_atraso` días tarde.
///
/// Devuelve `DomainError::ReglaFiscalNoEncontrada` mientras la regla esté
/// `por_definir`, no esté verificada, o no defina el `porcentaje_diario`
/// necesario. Nunca inventa porcentajes.
///
/// Cuando la regla sí está verificada, aplica
/// `monto * porcentaje_diario * dias_atraso`, recortado al `tope` si la regla
/// lo define.
pub fn estimar_multa(
    regla: &ReglaFiscal,
    monto: Decimal,
    dias_atraso: u32,
) -> Result<MultaEstimada, DomainError> {
    if esta_por_definir(regla) || !regla.verificada {
        return Err(DomainError::ReglaFiscalNoEncontrada(format!(
            "La regla '{}' no está verificada: no se estiman multas",
            regla.codigo
        )));
    }

    if monto < Decimal::ZERO {
        return Err(DomainError::ImporteInvalido);
    }

    let porcentaje_diario = decimal_desde(regla, "porcentaje_diario")?;
    let bruto = redondear(monto * porcentaje_diario * Decimal::from(dias_atraso));

    let monto_multa = match regla
        .parametros
        .get("tope")
        .and_then(|valor| valor.as_str())
    {
        Some(texto) => {
            let tope = texto.parse::<Decimal>().map_err(|_| {
                DomainError::ValidacionError(format!(
                    "Tope de multa inválido en '{}': '{}'",
                    regla.codigo, texto
                ))
            })?;
            bruto.min(tope)
        }
        None => bruto,
    };

    Ok(MultaEstimada {
        monto_multa,
        dias_atraso,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use serde_json::json;

    fn regla(verificada: bool, parametros: serde_json::Value) -> ReglaFiscal {
        ReglaFiscal {
            id: "regla:multa_extemporanea:v1".into(),
            rev: None,
            tipo: "regla_fiscal".into(),
            codigo: "multa_extemporanea".into(),
            vigencia_desde: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
            vigencia_hasta: None,
            fuente_normativa: "prueba".into(),
            version: 1,
            verificada,
            parametros,
        }
    }

    #[test]
    fn regla_por_definir_devuelve_error() {
        let regla = regla(false, json!({ "estado": "por_definir" }));
        assert!(matches!(
            estimar_multa(&regla, Decimal::new(1000, 0), 3),
            Err(DomainError::ReglaFiscalNoEncontrada(_))
        ));
    }

    #[test]
    fn regla_no_verificada_devuelve_error() {
        let regla = regla(false, json!({ "porcentaje_diario": "0.01" }));
        assert!(matches!(
            estimar_multa(&regla, Decimal::new(1000, 0), 3),
            Err(DomainError::ReglaFiscalNoEncontrada(_))
        ));
    }

    #[test]
    fn regla_verificada_sin_porcentaje_no_inventa() {
        let regla = regla(true, json!({}));
        assert!(matches!(
            estimar_multa(&regla, Decimal::new(1000, 0), 3),
            Err(DomainError::ReglaFiscalNoEncontrada(_))
        ));
    }

    #[test]
    fn regla_verificada_calcula_con_porcentaje_de_la_regla() {
        let regla = regla(true, json!({ "porcentaje_diario": "0.01" }));
        let multa = estimar_multa(&regla, Decimal::new(1000, 0), 3).unwrap();
        assert_eq!(multa.monto_multa, Decimal::new(30, 0));
        assert_eq!(multa.dias_atraso, 3);
    }

    #[test]
    fn regla_verificada_respeta_el_tope() {
        let regla = regla(
            true,
            json!({ "porcentaje_diario": "0.01", "tope": "20.00" }),
        );
        let multa = estimar_multa(&regla, Decimal::new(1000, 0), 3).unwrap();
        assert_eq!(multa.monto_multa, Decimal::new(20, 0));
    }
}
