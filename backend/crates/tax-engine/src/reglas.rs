use chrono::NaiveDate;
use domain::{DomainError, ReglaFiscal};
use rust_decimal::Decimal;

use crate::ParametrosFiscales;

/// Códigos de regla de los que se derivan los parámetros fiscales.
pub const CODIGO_IVA_GENERAL: &str = "iva_general";
pub const CODIGO_PAGO_CUENTA: &str = "pago_cuenta";

fn esta_vigente(regla: &ReglaFiscal, fecha: NaiveDate) -> bool {
    let hasta_ok = match regla.vigencia_hasta {
        Some(hasta) => fecha <= hasta,
        None => true,
    };
    regla.vigencia_desde <= fecha && hasta_ok
}

/// Una regla marcada como `por_definir` no es utilizable en cálculos.
pub(crate) fn esta_por_definir(regla: &ReglaFiscal) -> bool {
    regla
        .parametros
        .get("estado")
        .and_then(|valor| valor.as_str())
        .map(|estado| estado == "por_definir")
        .unwrap_or(false)
}

/// Selecciona la regla vigente en `fecha` para el `codigo` dado.
///
/// Si hay varias vigentes, devuelve la de mayor `version`. Las reglas con
/// `parametros.estado == "por_definir"` se descartan. Si no hay ninguna
/// utilizable, devuelve `DomainError::ReglaFiscalNoEncontrada`.
pub fn seleccionar_regla<'a>(
    reglas: &'a [ReglaFiscal],
    codigo: &str,
    fecha: NaiveDate,
) -> Result<&'a ReglaFiscal, DomainError> {
    reglas
        .iter()
        .filter(|regla| regla.codigo == codigo)
        .filter(|regla| esta_vigente(regla, fecha))
        .filter(|regla| !esta_por_definir(regla))
        .max_by_key(|regla| regla.version)
        .ok_or_else(|| DomainError::ReglaFiscalNoEncontrada(format!("{} @ {}", codigo, fecha)))
}

pub(crate) fn decimal_desde(regla: &ReglaFiscal, clave: &str) -> Result<Decimal, DomainError> {
    let valor = regla.parametros.get(clave).ok_or_else(|| {
        DomainError::ReglaFiscalNoEncontrada(format!("{}.{}", regla.codigo, clave))
    })?;

    let texto = valor.as_str().ok_or_else(|| {
        DomainError::ValidacionError(format!(
            "El parámetro '{}' de la regla '{}' debe ser un string Decimal",
            clave, regla.codigo
        ))
    })?;

    texto.parse::<Decimal>().map_err(|_| {
        DomainError::ValidacionError(format!(
            "Valor Decimal inválido en '{}.{}': '{}'",
            regla.codigo, clave, texto
        ))
    })
}

pub(crate) fn entero_desde(regla: &ReglaFiscal, clave: &str) -> Result<u32, DomainError> {
    let valor = regla.parametros.get(clave).ok_or_else(|| {
        DomainError::ReglaFiscalNoEncontrada(format!("{}.{}", regla.codigo, clave))
    })?;

    if let Some(numero) = valor.as_u64() {
        return u32::try_from(numero).map_err(|_| {
            DomainError::ValidacionError(format!(
                "El parámetro '{}.{}' excede el rango admitido",
                regla.codigo, clave
            ))
        });
    }

    if let Some(texto) = valor.as_str() {
        return texto.parse::<u32>().map_err(|_| {
            DomainError::ValidacionError(format!(
                "Valor entero inválido en '{}.{}': '{}'",
                regla.codigo, clave, texto
            ))
        });
    }

    Err(DomainError::ValidacionError(format!(
        "El parámetro '{}.{}' debe ser un entero",
        regla.codigo, clave
    )))
}

impl ParametrosFiscales {
    /// Deriva los parámetros fiscales a partir de las reglas vigentes en `fecha`.
    ///
    /// Lee `iva_general` y `pago_cuenta` (campo `tasa`, string Decimal). El
    /// plazo de presentación se resuelve por separado en `calendario`. Si falta
    /// alguna de las reglas o de los valores, devuelve
    /// `DomainError::ReglaFiscalNoEncontrada`.
    pub fn desde_reglas(reglas: &[ReglaFiscal], fecha: NaiveDate) -> Result<Self, DomainError> {
        let regla_iva = seleccionar_regla(reglas, CODIGO_IVA_GENERAL, fecha)?;
        let regla_pago = seleccionar_regla(reglas, CODIGO_PAGO_CUENTA, fecha)?;

        Ok(Self {
            tasa_iva: decimal_desde(regla_iva, "tasa")?,
            tasa_pago_a_cuenta: decimal_desde(regla_pago, "tasa")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fecha(texto: &str) -> NaiveDate {
        NaiveDate::parse_from_str(texto, "%Y-%m-%d").unwrap()
    }

    fn regla(
        codigo: &str,
        version: u32,
        desde: &str,
        hasta: Option<&str>,
        parametros: serde_json::Value,
    ) -> ReglaFiscal {
        ReglaFiscal {
            id: format!("regla:{}:v{}", codigo, version),
            rev: None,
            tipo: "regla_fiscal".into(),
            codigo: codigo.into(),
            vigencia_desde: fecha(desde),
            vigencia_hasta: hasta.map(fecha),
            fuente_normativa: "prueba".into(),
            version,
            verificada: true,
            parametros,
        }
    }

    fn reglas_completas() -> Vec<ReglaFiscal> {
        vec![
            regla(
                "iva_general",
                1,
                "2020-01-01",
                None,
                json!({ "tasa": "0.13" }),
            ),
            regla(
                "pago_cuenta",
                1,
                "2020-01-01",
                None,
                json!({ "tasa": "0.0175" }),
            ),
        ]
    }

    #[test]
    fn cambio_de_vigencia_a_mitad_de_anio() {
        let reglas = vec![
            regla(
                "iva_general",
                1,
                "2020-01-01",
                Some("2026-06-30"),
                json!({ "tasa": "0.13" }),
            ),
            regla(
                "iva_general",
                1,
                "2026-07-01",
                None,
                json!({ "tasa": "0.15" }),
            ),
        ];

        let antes = seleccionar_regla(&reglas, "iva_general", fecha("2026-05-15")).unwrap();
        assert_eq!(decimal_desde(antes, "tasa").unwrap(), Decimal::new(13, 2));

        let despues = seleccionar_regla(&reglas, "iva_general", fecha("2026-08-01")).unwrap();
        assert_eq!(decimal_desde(despues, "tasa").unwrap(), Decimal::new(15, 2));
    }

    #[test]
    fn regla_faltante_devuelve_error() {
        let reglas = reglas_completas();
        assert!(matches!(
            seleccionar_regla(&reglas, "renta_juridica", fecha("2026-05-01")),
            Err(DomainError::ReglaFiscalNoEncontrada(_))
        ));

        let sin_pago: Vec<ReglaFiscal> = reglas
            .into_iter()
            .filter(|r| r.codigo != "pago_cuenta")
            .collect();
        assert!(matches!(
            ParametrosFiscales::desde_reglas(&sin_pago, fecha("2026-05-01")),
            Err(DomainError::ReglaFiscalNoEncontrada(_))
        ));
    }

    #[test]
    fn version_dos_tiene_prioridad_sobre_version_uno() {
        let reglas = vec![
            regla(
                "iva_general",
                1,
                "2020-01-01",
                None,
                json!({ "tasa": "0.13" }),
            ),
            regla(
                "iva_general",
                2,
                "2020-01-01",
                None,
                json!({ "tasa": "0.15" }),
            ),
        ];

        let seleccionada = seleccionar_regla(&reglas, "iva_general", fecha("2026-05-01")).unwrap();
        assert_eq!(seleccionada.version, 2);
        assert_eq!(
            decimal_desde(seleccionada, "tasa").unwrap(),
            Decimal::new(15, 2)
        );
    }

    #[test]
    fn regla_por_definir_no_es_seleccionable() {
        let reglas = vec![regla(
            "multa_extemporanea",
            1,
            "2020-01-01",
            None,
            json!({ "estado": "por_definir" }),
        )];

        assert!(matches!(
            seleccionar_regla(&reglas, "multa_extemporanea", fecha("2026-05-01")),
            Err(DomainError::ReglaFiscalNoEncontrada(_))
        ));
    }

    #[test]
    fn desde_reglas_parsea_strings_decimales() {
        let params =
            ParametrosFiscales::desde_reglas(&reglas_completas(), fecha("2026-05-01")).unwrap();
        assert_eq!(params.tasa_iva, Decimal::new(13, 2));
        assert_eq!(params.tasa_pago_a_cuenta, Decimal::new(175, 4));
    }

    #[test]
    fn parametro_faltante_devuelve_error() {
        let mut reglas = reglas_completas();
        reglas[0].parametros = json!({ "otra": "0.13" });

        assert!(matches!(
            ParametrosFiscales::desde_reglas(&reglas, fecha("2026-05-01")),
            Err(DomainError::ReglaFiscalNoEncontrada(_))
        ));
    }
}
