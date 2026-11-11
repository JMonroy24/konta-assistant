//! Cálculo del impuesto sobre la renta para persona natural (tabla progresiva
//! por tramos) y persona jurídica (tasa general o reducida según umbral).

use chrono::NaiveDate;
use domain::{DomainError, ReglaFiscal};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::explicacion;
use crate::redondear;
use crate::reglas::decimal_desde;

/// Código de la regla con la tabla progresiva de renta para persona natural.
pub const CODIGO_RENTA_NATURAL_TRAMOS: &str = "renta_natural_tramos";
/// Código de la regla con las tasas de renta para persona jurídica.
pub const CODIGO_RENTA_JURIDICA: &str = "renta_juridica";

/// Resultado del cálculo de renta, con el saldo tras restar los pagos a cuenta.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultadoRenta {
    pub impuesto_causado: Decimal,
    pub pagos_a_cuenta: Decimal,
    /// `impuesto_causado - pagos_a_cuenta`. Negativo = saldo a favor.
    pub saldo: Decimal,
    pub explicacion: String,
}

/// Tramo de la tabla progresiva de renta para persona natural.
#[derive(Debug, Clone)]
pub struct Tramo {
    pub desde: Decimal,
    pub hasta: Option<Decimal>,
    pub tasa: Decimal,
    pub cuota_fija: Decimal,
    pub exceso_sobre: Decimal,
}

fn decimal_de_valor(valor: &Value, campo: &str, contexto: &str) -> Result<Decimal, DomainError> {
    let texto = valor.get(campo).and_then(Value::as_str).ok_or_else(|| {
        DomainError::ValidacionError(format!(
            "El campo '{}' de {} debe ser un string Decimal",
            campo, contexto
        ))
    })?;

    texto.parse::<Decimal>().map_err(|_| {
        DomainError::ValidacionError(format!(
            "Valor Decimal inválido en {}.{}: '{}'",
            contexto, campo, texto
        ))
    })
}

fn tramos_desde(regla: &ReglaFiscal) -> Result<Vec<Tramo>, DomainError> {
    let tramos = regla
        .parametros
        .get("tramos")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            DomainError::ValidacionError(format!(
                "La regla '{}' no define el arreglo 'tramos'",
                regla.codigo
            ))
        })?;

    let contexto = format!("{}.tramos", regla.codigo);
    let mut resultado = Vec::with_capacity(tramos.len());
    for tramo in tramos {
        let hasta = match tramo.get("hasta") {
            None | Some(Value::Null) => None,
            Some(_) => Some(decimal_de_valor(tramo, "hasta", &contexto)?),
        };
        resultado.push(Tramo {
            desde: decimal_de_valor(tramo, "desde", &contexto)?,
            hasta,
            tasa: decimal_de_valor(tramo, "tasa", &contexto)?,
            cuota_fija: decimal_de_valor(tramo, "cuota_fija", &contexto)?,
            exceso_sobre: decimal_de_valor(tramo, "exceso_sobre", &contexto)?,
        });
    }
    Ok(resultado)
}

fn tramo_para(tramos: &[Tramo], renta: Decimal) -> Option<&Tramo> {
    tramos.iter().find(|tramo| {
        renta >= tramo.desde
            && match tramo.hasta {
                Some(hasta) => renta <= hasta,
                None => true,
            }
    })
}

/// Calcula el impuesto de renta de una persona natural aplicando la tabla
/// progresiva `cuota_fija + tasa * (renta - exceso_sobre)`.
///
/// La renta por debajo del primer tramo no causa impuesto. A la renta negativa
/// se le rechaza con `DomainError::ImporteInvalido`.
pub fn renta_persona_natural(
    renta_neta: Decimal,
    regla: &ReglaFiscal,
    pagos_a_cuenta: Decimal,
    fecha_limite: NaiveDate,
) -> Result<ResultadoRenta, DomainError> {
    if renta_neta < Decimal::ZERO || pagos_a_cuenta < Decimal::ZERO {
        return Err(DomainError::ImporteInvalido);
    }

    let tramos = tramos_desde(regla)?;
    let tramo = tramo_para(&tramos, renta_neta);
    let impuesto_causado = match tramo {
        Some(tramo) => redondear(tramo.cuota_fija + tramo.tasa * (renta_neta - tramo.exceso_sobre)),
        None => Decimal::ZERO,
    };
    let saldo = redondear(impuesto_causado - pagos_a_cuenta);
    let explicacion = explicacion::explicacion_renta_natural(
        renta_neta,
        tramo,
        impuesto_causado,
        pagos_a_cuenta,
        saldo,
        fecha_limite,
    );

    Ok(ResultadoRenta {
        impuesto_causado,
        pagos_a_cuenta,
        saldo,
        explicacion,
    })
}

/// Calcula el impuesto de renta de una persona jurídica.
///
/// Aplica `tasa_reducida` cuando los ingresos anuales no superan el umbral, o
/// `tasa_general` en caso contrario. A la renta negativa se le rechaza con
/// `DomainError::ImporteInvalido`.
pub fn renta_persona_juridica(
    ingresos_anuales: Decimal,
    renta_neta: Decimal,
    regla: &ReglaFiscal,
    pagos_a_cuenta: Decimal,
    fecha_limite: NaiveDate,
) -> Result<ResultadoRenta, DomainError> {
    if ingresos_anuales < Decimal::ZERO
        || renta_neta < Decimal::ZERO
        || pagos_a_cuenta < Decimal::ZERO
    {
        return Err(DomainError::ImporteInvalido);
    }

    let umbral = decimal_desde(regla, "umbral_ingresos_anuales")?;
    let es_reducida = ingresos_anuales <= umbral;
    let tasa = if es_reducida {
        decimal_desde(regla, "tasa_reducida")?
    } else {
        decimal_desde(regla, "tasa_general")?
    };

    let impuesto_causado = redondear(renta_neta * tasa);
    let saldo = redondear(impuesto_causado - pagos_a_cuenta);
    let explicacion = explicacion::explicacion_renta_juridica(
        ingresos_anuales,
        renta_neta,
        umbral,
        tasa,
        es_reducida,
        impuesto_causado,
        pagos_a_cuenta,
        saldo,
        fecha_limite,
    );

    Ok(ResultadoRenta {
        impuesto_causado,
        pagos_a_cuenta,
        saldo,
        explicacion,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use serde_json::json;

    fn fecha(texto: &str) -> NaiveDate {
        NaiveDate::parse_from_str(texto, "%Y-%m-%d").expect("fecha de prueba válida")
    }

    fn regla_natural() -> ReglaFiscal {
        ReglaFiscal {
            id: "regla:renta_natural_tramos:v1".into(),
            rev: None,
            tipo: "regla_fiscal".into(),
            codigo: CODIGO_RENTA_NATURAL_TRAMOS.into(),
            vigencia_desde: fecha("2020-01-01"),
            vigencia_hasta: None,
            fuente_normativa: "prueba".into(),
            version: 1,
            verificada: true,
            parametros: json!({
                "tramos": [
                    { "desde": "0.01",     "hasta": "4064.00",  "tasa": "0.00", "cuota_fija": "0.00",    "exceso_sobre": "0.00" },
                    { "desde": "4064.01",  "hasta": "9142.86",  "tasa": "0.10", "cuota_fija": "212.12",  "exceso_sobre": "4064.00" },
                    { "desde": "9142.87",  "hasta": "22857.14", "tasa": "0.20", "cuota_fija": "720.00",  "exceso_sobre": "9142.86" },
                    { "desde": "22857.15", "hasta": null,       "tasa": "0.30", "cuota_fija": "3462.86", "exceso_sobre": "22857.14" }
                ]
            }),
        }
    }

    fn regla_juridica() -> ReglaFiscal {
        ReglaFiscal {
            id: "regla:renta_juridica:v1".into(),
            rev: None,
            tipo: "regla_fiscal".into(),
            codigo: CODIGO_RENTA_JURIDICA.into(),
            vigencia_desde: fecha("2020-01-01"),
            vigencia_hasta: None,
            fuente_normativa: "prueba".into(),
            version: 1,
            verificada: true,
            parametros: json!({
                "tasa_general": "0.30",
                "tasa_reducida": "0.25",
                "umbral_ingresos_anuales": "150000.00"
            }),
        }
    }

    #[test]
    fn un_valor_por_tramo() {
        let regla = regla_natural();
        let limite = fecha("2027-04-30");
        let casos = [
            (dec!(2000.00), dec!(0.00)),
            (dec!(6000.00), dec!(405.72)),
            (dec!(15000.00), dec!(1891.43)),
            (dec!(30000.00), dec!(5605.72)),
        ];
        for (renta, esperado) in casos {
            let resultado = renta_persona_natural(renta, &regla, Decimal::ZERO, limite).unwrap();
            assert_eq!(resultado.impuesto_causado, esperado, "renta {renta}");
        }
    }

    #[test]
    fn en_el_borde_de_cada_tramo() {
        let regla = regla_natural();
        let limite = fecha("2027-04-30");
        let casos = [
            (dec!(0.01), dec!(0.00)),
            (dec!(4064.00), dec!(0.00)),
            (dec!(4064.01), dec!(212.12)),
            (dec!(9142.86), dec!(720.01)),
            (dec!(9142.87), dec!(720.00)),
            (dec!(22857.14), dec!(3462.86)),
            (dec!(22857.15), dec!(3462.86)),
        ];
        for (renta, esperado) in casos {
            let resultado = renta_persona_natural(renta, &regla, Decimal::ZERO, limite).unwrap();
            assert_eq!(resultado.impuesto_causado, esperado, "renta {renta}");
        }
    }

    #[test]
    fn renta_cero_no_causa_impuesto() {
        let regla = regla_natural();
        let resultado =
            renta_persona_natural(Decimal::ZERO, &regla, Decimal::ZERO, fecha("2027-04-30"))
                .unwrap();
        assert_eq!(resultado.impuesto_causado, Decimal::ZERO);
        assert_eq!(resultado.saldo, Decimal::ZERO);
        assert!(resultado.explicacion.contains("no se te cobra renta"));
    }

    #[test]
    fn resta_pagos_a_cuenta_para_obtener_saldo() {
        let regla = regla_natural();
        let limite = fecha("2027-04-30");
        let a_pagar = renta_persona_natural(dec!(6000.00), &regla, dec!(100.00), limite).unwrap();
        assert_eq!(a_pagar.impuesto_causado, dec!(405.72));
        assert_eq!(a_pagar.saldo, dec!(305.72));
        assert!(a_pagar.explicacion.contains("te falta pagar"));

        let a_favor = renta_persona_natural(dec!(6000.00), &regla, dec!(500.00), limite).unwrap();
        assert_eq!(a_favor.saldo, dec!(-94.28));
        assert!(a_favor.explicacion.contains("te queda a favor"));
    }

    #[test]
    fn juridica_bajo_umbral_usa_tasa_reducida() {
        let regla = regla_juridica();
        let resultado = renta_persona_juridica(
            dec!(100000.00),
            dec!(50000.00),
            &regla,
            Decimal::ZERO,
            fecha("2027-04-30"),
        )
        .unwrap();
        assert_eq!(resultado.impuesto_causado, dec!(12500.00));
        assert!(resultado.explicacion.contains("reducida"));
    }

    #[test]
    fn juridica_justo_en_umbral_usa_tasa_reducida() {
        let regla = regla_juridica();
        let resultado = renta_persona_juridica(
            dec!(150000.00),
            dec!(50000.00),
            &regla,
            Decimal::ZERO,
            fecha("2027-04-30"),
        )
        .unwrap();
        assert_eq!(resultado.impuesto_causado, dec!(12500.00));
    }

    #[test]
    fn juridica_sobre_umbral_usa_tasa_general() {
        let regla = regla_juridica();
        let resultado = renta_persona_juridica(
            dec!(200000.00),
            dec!(50000.00),
            &regla,
            Decimal::ZERO,
            fecha("2027-04-30"),
        )
        .unwrap();
        assert_eq!(resultado.impuesto_causado, dec!(15000.00));
        assert!(resultado.explicacion.contains("general"));
    }

    #[test]
    fn renta_negativa_es_rechazada() {
        let regla = regla_natural();
        assert!(matches!(
            renta_persona_natural(dec!(-1.00), &regla, Decimal::ZERO, fecha("2027-04-30")),
            Err(DomainError::ImporteInvalido)
        ));
    }
}
