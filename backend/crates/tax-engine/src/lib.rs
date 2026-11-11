pub mod calendario;
pub mod explicacion;
pub mod multas;
pub mod reglas;
pub mod renta;

use domain::{DomainError, Operacion, Periodo, ResumenImpuestos, TipoOperacion};
use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParametrosFiscales {
    pub tasa_iva: Decimal,           // Ej: 0.13
    pub tasa_pago_a_cuenta: Decimal, // Ej: 0.0175 (1.75%)
}

#[cfg(test)]
impl Default for ParametrosFiscales {
    fn default() -> Self {
        Self {
            tasa_iva: Decimal::new(13, 2),
            tasa_pago_a_cuenta: Decimal::new(175, 4),
        }
    }
}

/// Redondeo half-up (alejado de cero) a 2 decimales.
pub(crate) fn redondear(valor: Decimal) -> Decimal {
    valor.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
}

pub struct MotorFiscal;

impl MotorFiscal {
    /// Calcula el resumen de IVA y pago a cuenta de un período.
    ///
    /// Solo considera las operaciones cuya fecha cae dentro del período. Un
    /// `remanente_periodo_anterior` se descuenta del IVA a pagar antes de
    /// determinar el remanente del período.
    pub fn calcular_impuestos(
        periodo: Periodo,
        operaciones: &[Operacion],
        params: &ParametrosFiscales,
        remanente_periodo_anterior: Option<Decimal>,
    ) -> Result<ResumenImpuestos, DomainError> {
        let (inicio, fin) = periodo.rango_fechas()?;

        let mut ingresos_gravados = Decimal::ZERO;
        let mut debito = Decimal::ZERO;
        let mut credito = Decimal::ZERO;

        for op in operaciones
            .iter()
            .filter(|op| op.fecha >= inicio && op.fecha <= fin)
        {
            match op.tipo {
                TipoOperacion::Venta => {
                    debito += op.iva;
                    ingresos_gravados += op.base;
                }
                TipoOperacion::Compra => {
                    credito += op.iva;
                }
            }
        }

        let ingresos_gravados = redondear(ingresos_gravados);
        let debito = redondear(debito);
        let credito = redondear(credito);
        let pago_a_cuenta = redondear(ingresos_gravados * params.tasa_pago_a_cuenta);

        let remanente_anterior = remanente_periodo_anterior.unwrap_or(Decimal::ZERO);
        let neto = redondear(debito - credito - remanente_anterior);

        let (iva_a_pagar, remanente) = if neto > Decimal::ZERO {
            (neto, Decimal::ZERO)
        } else {
            (Decimal::ZERO, -neto)
        };

        Ok(ResumenImpuestos {
            periodo,
            ingresos_gravados,
            debito,
            credito,
            iva_a_pagar,
            remanente,
            remanente_periodo_anterior: remanente_periodo_anterior.map(redondear),
            pago_a_cuenta,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, Utc};
    use rust_decimal_macros::dec;

    fn operacion(tipo: TipoOperacion, fecha: &str, base: Decimal, iva: Decimal) -> Operacion {
        let fecha = NaiveDate::parse_from_str(fecha, "%Y-%m-%d").expect("fecha de prueba válida");
        Operacion {
            id: format!("op-{}", fecha),
            rev: None,
            tipo,
            fecha,
            descripcion: "Operación de prueba".into(),
            base,
            iva,
            total: base + iva,
            contraparte: "Contraparte".into(),
            creado_por: "usr1".into(),
            origen: "local".into(),
            creado_en: Utc::now(),
        }
    }

    fn periodo(texto: &str) -> Periodo {
        Periodo::parse(texto).expect("período de prueba válido")
    }

    #[test]
    fn test_iva_a_pagar_con_valores_conocidos() {
        let params = ParametrosFiscales::default();
        let venta = operacion(
            TipoOperacion::Venta,
            "2026-11-05",
            dec!(100.00),
            dec!(13.00),
        );

        let resumen = MotorFiscal::calcular_impuestos(periodo("2026-11"), &[venta], &params, None)
            .expect("cálculo válido");

        assert_eq!(resumen.ingresos_gravados, dec!(100.00));
        assert_eq!(resumen.debito, dec!(13.00));
        assert_eq!(resumen.credito, Decimal::ZERO);
        assert_eq!(resumen.iva_a_pagar, dec!(13.00));
        assert_eq!(resumen.remanente, Decimal::ZERO);
        assert_eq!(resumen.pago_a_cuenta, dec!(1.75));
    }

    #[test]
    fn test_ventas_con_redondeo_half_up() {
        let params = ParametrosFiscales::default();
        // 100.005 -> 100.01 y 13.005 -> 13.01 (half-up, alejado de cero).
        let venta = operacion(
            TipoOperacion::Venta,
            "2026-11-10",
            dec!(100.005),
            dec!(13.005),
        );

        let resumen = MotorFiscal::calcular_impuestos(periodo("2026-11"), &[venta], &params, None)
            .expect("cálculo válido");

        assert_eq!(resumen.ingresos_gravados, dec!(100.01));
        assert_eq!(resumen.debito, dec!(13.01));
        assert_eq!(resumen.iva_a_pagar, dec!(13.01));
    }

    #[test]
    fn test_solo_compras_genera_remanente() {
        let params = ParametrosFiscales::default();
        let compra = operacion(TipoOperacion::Compra, "2026-11-07", dec!(50.00), dec!(6.50));

        let resumen = MotorFiscal::calcular_impuestos(periodo("2026-11"), &[compra], &params, None)
            .expect("cálculo válido");

        assert_eq!(resumen.ingresos_gravados, Decimal::ZERO);
        assert_eq!(resumen.debito, Decimal::ZERO);
        assert_eq!(resumen.credito, dec!(6.50));
        assert_eq!(resumen.iva_a_pagar, Decimal::ZERO);
        assert_eq!(resumen.remanente, dec!(6.50));
    }

    #[test]
    fn test_periodo_vacio() {
        let params = ParametrosFiscales::default();

        let resumen = MotorFiscal::calcular_impuestos(periodo("2026-11"), &[], &params, None)
            .expect("cálculo válido");

        assert_eq!(resumen.ingresos_gravados, Decimal::ZERO);
        assert_eq!(resumen.debito, Decimal::ZERO);
        assert_eq!(resumen.credito, Decimal::ZERO);
        assert_eq!(resumen.iva_a_pagar, Decimal::ZERO);
        assert_eq!(resumen.remanente, Decimal::ZERO);
        assert_eq!(resumen.pago_a_cuenta, Decimal::ZERO);
    }

    #[test]
    fn test_mil_operaciones() {
        let params = ParametrosFiscales::default();
        let operaciones: Vec<Operacion> = (0..1000)
            .map(|i| {
                let dia = (i % 28) + 1;
                let fecha = format!("2026-11-{dia:02}");
                operacion(TipoOperacion::Venta, &fecha, dec!(100.00), dec!(13.00))
            })
            .collect();

        let resumen =
            MotorFiscal::calcular_impuestos(periodo("2026-11"), &operaciones, &params, None)
                .expect("cálculo válido");

        assert_eq!(resumen.ingresos_gravados, dec!(100000.00));
        assert_eq!(resumen.debito, dec!(13000.00));
        assert_eq!(resumen.iva_a_pagar, dec!(13000.00));
        assert_eq!(resumen.pago_a_cuenta, dec!(1750.00));
    }

    #[test]
    fn test_remanente_periodo_anterior_se_descuenta() {
        let params = ParametrosFiscales::default();
        let venta = operacion(
            TipoOperacion::Venta,
            "2026-11-05",
            dec!(100.00),
            dec!(13.00),
        );

        let resumen = MotorFiscal::calcular_impuestos(
            periodo("2026-11"),
            &[venta],
            &params,
            Some(dec!(5.00)),
        )
        .expect("cálculo válido");

        assert_eq!(resumen.iva_a_pagar, dec!(8.00));
        assert_eq!(resumen.remanente, Decimal::ZERO);
        assert_eq!(resumen.remanente_periodo_anterior, Some(dec!(5.00)));
    }

    #[test]
    fn test_remanente_anterior_mayor_que_debito_genera_remanente() {
        let params = ParametrosFiscales::default();
        let venta = operacion(
            TipoOperacion::Venta,
            "2026-11-05",
            dec!(100.00),
            dec!(13.00),
        );

        let resumen = MotorFiscal::calcular_impuestos(
            periodo("2026-11"),
            &[venta],
            &params,
            Some(dec!(20.00)),
        )
        .expect("cálculo válido");

        assert_eq!(resumen.iva_a_pagar, Decimal::ZERO);
        assert_eq!(resumen.remanente, dec!(7.00));
    }

    #[test]
    fn test_ignora_operaciones_de_otro_periodo() {
        let params = ParametrosFiscales::default();
        let de_octubre = operacion(
            TipoOperacion::Venta,
            "2026-10-31",
            dec!(100.00),
            dec!(13.00),
        );
        let de_noviembre = operacion(
            TipoOperacion::Venta,
            "2026-11-01",
            dec!(200.00),
            dec!(26.00),
        );

        let resumen = MotorFiscal::calcular_impuestos(
            periodo("2026-11"),
            &[de_octubre, de_noviembre],
            &params,
            None,
        )
        .expect("cálculo válido");

        assert_eq!(resumen.ingresos_gravados, dec!(200.00));
        assert_eq!(resumen.debito, dec!(26.00));
        assert_eq!(resumen.iva_a_pagar, dec!(26.00));
    }
}
