use chrono::NaiveDate;
use domain::{DomainError, Operacion, ResumenImpuestos, TipoOperacion};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParametrosFiscales {
    pub tasa_iva: Decimal,            // Ej: 0.13
    pub tasa_pago_a_cuenta: Decimal,   // Ej: 0.0175 (1.75%)
    pub dia_limite_pago: u32,         // Ej: 10 de cada mes
}

impl Default for ParametrosFiscales {
    fn default() -> Self {
        Self {
            tasa_iva: dec!(0.13),
            tasa_pago_a_cuenta: dec!(0.0175),
            dia_limite_pago: 10,
        }
    }
}

pub struct MotorFiscal;

impl MotorFiscal {
    pub fn calcular_impuestos(
        periodo: &str,
        operaciones: &[Operacion],
        params: &ParametrosFiscales,
    ) -> Result<ResumenImpuestos, DomainError> {
        let mut debito_iva = Decimal::ZERO;
        let mut credito_iva = Decimal::ZERO;
        let mut ingresos_brutos = Decimal::ZERO;

        for op in operaciones {
            match op.tipo {
                TipoOperacion::Venta => {
                    debito_iva += op.iva;
                    ingresos_brutos += op.base;
                }
                TipoOperacion::Compra => {
                    credito_iva += op.iva;
                }
            }
        }

        let (iva_a_pagar, iva_remanente) = if debito_iva >= credito_iva {
            (debito_iva - credito_iva, Decimal::ZERO)
        } else {
            (Decimal::ZERO, credito_iva - debito_iva)
        };

        // Pago a cuenta = ingresos gravados * tasa (ej: 1.75%)
        let pago_a_cuenta = (ingresos_brutos * params.tasa_pago_a_cuenta).round_dp(2);

        // Parse año y mes para fecha límite
        let parts: Vec<&str> = periodo.split('-').collect();
        let (anio, mes) = if parts.len() == 2 {
            let a: i32 = parts[0].parse().unwrap_or(2026);
            let m: u32 = parts[1].parse().unwrap_or(1);
            let next_m = if m == 12 { 1 } else { m + 1 };
            let next_a = if m == 12 { a + 1 } else { a };
            (next_a, next_m)
        } else {
            (2026, 2)
        };

        let fecha_limite = NaiveDate::from_ymd_opt(anio, mes, params.dia_limite_pago)
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(2026, 2, 10).unwrap());

        let explicacion = format!(
            "Período {}: Débito IVA acumulado (${:.2}) menos Crédito IVA (${:.2}). \
            Resultado IVA: ${:.2} {}. Pago a cuenta ({:.2}% de ${:.2}): ${:.2}.",
            periodo,
            debito_iva,
            credito_iva,
            if iva_a_pagar > Decimal::ZERO { iva_a_pagar } else { iva_remanente },
            if iva_a_pagar > Decimal::ZERO { "a pagar" } else { "remanente a favor" },
            params.tasa_pago_a_cuenta * dec!(100),
            ingresos_brutos,
            pago_a_cuenta
        );

        Ok(ResumenImpuestos {
            periodo: periodo.to_string(),
            debito_iva,
            credito_iva,
            iva_a_pagar,
            iva_remanente,
            pago_a_cuenta,
            fecha_limite_pago: fecha_limite,
            explicacion_calculo: explicacion,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_calculo_periodo_sin_movimientos() {
        let params = ParametrosFiscales::default();
        let resumen = MotorFiscal::calcular_impuestos("2026-11", &[], &params).unwrap();
        assert_eq!(resumen.iva_a_pagar, Decimal::ZERO);
        assert_eq!(resumen.iva_remanente, Decimal::ZERO);
        assert_eq!(resumen.pago_a_cuenta, Decimal::ZERO);
    }

    #[test]
    fn test_calculo_con_iva_a_pagar() {
        let params = ParametrosFiscales::default();
        let venta = Operacion {
            id: "v-1".into(),
            rev: None,
            tipo: TipoOperacion::Venta,
            fecha: NaiveDate::from_ymd_opt(2026, 11, 5).unwrap(),
            descripcion: "Venta servicio".into(),
            base: dec!(100.00),
            iva: dec!(13.00),
            total: dec!(113.00),
            contraparte: "Cliente A".into(),
            creado_por: "usr1".into(),
            origen: "local".into(),
            creado_en: Utc::now(),
        };
        let compra = Operacion {
            id: "c-1".into(),
            rev: None,
            tipo: TipoOperacion::Compra,
            fecha: NaiveDate::from_ymd_opt(2026, 11, 6).unwrap(),
            descripcion: "Compra insumos".into(),
            base: dec!(50.00),
            iva: dec!(6.50),
            total: dec!(56.50),
            contraparte: "Proveedor B".into(),
            creado_por: "usr1".into(),
            origen: "local".into(),
            creado_en: Utc::now(),
        };

        let resumen = MotorFiscal::calcular_impuestos("2026-11", &[venta, compra], &params).unwrap();
        assert_eq!(resumen.debito_iva, dec!(13.00));
        assert_eq!(resumen.credito_iva, dec!(6.50));
        assert_eq!(resumen.iva_a_pagar, dec!(6.50));
        assert_eq!(resumen.iva_remanente, Decimal::ZERO);
        assert_eq!(resumen.pago_a_cuenta, dec!(1.75));
    }
}
