//! Textos explicativos en español simple para las obligaciones fiscales.
//!
//! Todas las funciones devuelven lenguaje llano (sin jerga), incluyendo montos
//! y fechas, para mostrárselos al usuario final.

use chrono::NaiveDate;
use domain::ResumenImpuestos;
use rust_decimal::Decimal;

use crate::redondear;
use crate::renta::Tramo;

fn dinero(valor: Decimal) -> String {
    let mut redondeado = redondear(valor);
    redondeado.rescale(2);
    redondeado.to_string()
}

fn porcentaje(tasa: Decimal) -> String {
    let mut valor = redondear(tasa * Decimal::from(100));
    valor.rescale(2);
    valor.to_string()
}

fn fecha_es(fecha: NaiveDate) -> String {
    fecha.format("%d/%m/%Y").to_string()
}

fn cierre_saldo(pagos_a_cuenta: Decimal, saldo: Decimal) -> String {
    if saldo > Decimal::ZERO {
        format!(
            "Ya pagaste ${} a cuenta, así que te falta pagar ${}.",
            dinero(pagos_a_cuenta),
            dinero(saldo)
        )
    } else if saldo < Decimal::ZERO {
        format!(
            "Ya pagaste ${} a cuenta, así que te queda a favor ${}.",
            dinero(pagos_a_cuenta),
            dinero(-saldo)
        )
    } else {
        format!(
            "Lo que pagaste a cuenta (${}) cubre exactamente el impuesto.",
            dinero(pagos_a_cuenta)
        )
    }
}

/// Explicación del IVA mensual (formulario F07).
pub fn explicacion_iva(resumen: &ResumenImpuestos, fecha_limite: NaiveDate) -> String {
    let mut texto = format!(
        "En el mes {} el IVA de tus ventas fue ${} y el de tus compras ${}.",
        resumen.periodo,
        dinero(resumen.debito),
        dinero(resumen.credito),
    );

    if resumen.iva_a_pagar > Decimal::ZERO {
        texto.push_str(&format!(
            " Te toca pagar ${} de IVA.",
            dinero(resumen.iva_a_pagar)
        ));
    } else if resumen.remanente > Decimal::ZERO {
        texto.push_str(&format!(
            " No pagas IVA este mes y te queda a favor ${} para el próximo.",
            dinero(resumen.remanente)
        ));
    } else {
        texto.push_str(" No te queda IVA por pagar este mes.");
    }

    if let Some(anterior) = resumen.remanente_periodo_anterior {
        if anterior > Decimal::ZERO {
            texto.push_str(&format!(
                " Se descontaron ${} que tenías a favor del mes anterior.",
                dinero(anterior)
            ));
        }
    }

    texto.push_str(&format!(
        " La fecha límite para declarar y pagar es el {}.",
        fecha_es(fecha_limite)
    ));
    texto
}

/// Explicación del pago a cuenta mensual del impuesto sobre la renta (F14).
pub fn explicacion_pago_a_cuenta(resumen: &ResumenImpuestos, fecha_limite: NaiveDate) -> String {
    format!(
        "El pago a cuenta del impuesto sobre la renta del mes {} es ${}, calculado sobre tus ventas de ${}. La fecha límite para pagar es el {}.",
        resumen.periodo,
        dinero(resumen.pago_a_cuenta),
        dinero(resumen.ingresos_gravados),
        fecha_es(fecha_limite),
    )
}

/// Explicación de la renta anual de persona natural.
pub fn explicacion_renta_natural(
    renta_neta: Decimal,
    tramo: Option<&Tramo>,
    impuesto_causado: Decimal,
    pagos_a_cuenta: Decimal,
    saldo: Decimal,
    fecha_limite: NaiveDate,
) -> String {
    let base = match tramo {
        Some(tramo) => {
            let rango = match tramo.hasta {
                Some(hasta) => format!("entre ${} y ${}", dinero(tramo.desde), dinero(hasta)),
                None => format!("de ${} en adelante", dinero(tramo.desde)),
            };
            format!(
                "Tu renta neta de ${} cae en el tramo {}. Se calcula así: ${} fijos más el {}% de lo que supera los ${}, lo que da ${} de impuesto.",
                dinero(renta_neta),
                rango,
                dinero(tramo.cuota_fija),
                porcentaje(tramo.tasa),
                dinero(tramo.exceso_sobre),
                dinero(impuesto_causado),
            )
        }
        None => format!(
            "Tu renta neta de ${} está por debajo del monto mínimo que paga impuesto, así que no se te cobra renta.",
            dinero(renta_neta)
        ),
    };

    format!(
        "{} {} La fecha límite para declarar y pagar es el {}.",
        base,
        cierre_saldo(pagos_a_cuenta, saldo),
        fecha_es(fecha_limite),
    )
}

/// Explicación de la renta anual de persona jurídica.
#[allow(clippy::too_many_arguments)]
pub fn explicacion_renta_juridica(
    ingresos_anuales: Decimal,
    renta_neta: Decimal,
    umbral: Decimal,
    tasa: Decimal,
    es_reducida: bool,
    impuesto_causado: Decimal,
    pagos_a_cuenta: Decimal,
    saldo: Decimal,
    fecha_limite: NaiveDate,
) -> String {
    let comparacion = if es_reducida { "no superan" } else { "superan" };
    let tipo = if es_reducida { "reducida" } else { "general" };

    format!(
        "Tus ingresos anuales fueron ${}. Como {} el límite de ${}, se aplica la tasa {} del {}%. Sobre una renta neta de ${} el impuesto es ${}. {} La fecha límite para declarar y pagar es el {}.",
        dinero(ingresos_anuales),
        comparacion,
        dinero(umbral),
        tipo,
        porcentaje(tasa),
        dinero(renta_neta),
        dinero(impuesto_causado),
        cierre_saldo(pagos_a_cuenta, saldo),
        fecha_es(fecha_limite),
    )
}

/// Explicación de la multa por presentación extemporánea.
pub fn explicacion_multa(monto: Decimal, dias_atraso: u32, fecha_limite: NaiveDate) -> String {
    format!(
        "Presentaste tu declaración {} día(s) después de la fecha límite ({}). La multa estimada es ${}.",
        dias_atraso,
        fecha_es(fecha_limite),
        dinero(monto),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::Periodo;
    use rust_decimal_macros::dec;

    fn fecha(texto: &str) -> NaiveDate {
        NaiveDate::parse_from_str(texto, "%Y-%m-%d").expect("fecha de prueba válida")
    }

    fn resumen() -> ResumenImpuestos {
        ResumenImpuestos {
            periodo: Periodo::parse("2026-11").expect("período válido"),
            ingresos_gravados: dec!(100.00),
            debito: dec!(13.00),
            credito: dec!(6.50),
            iva_a_pagar: dec!(6.50),
            remanente: dec!(0.00),
            remanente_periodo_anterior: None,
            pago_a_cuenta: dec!(1.75),
        }
    }

    #[test]
    fn explicacion_iva_incluye_montos_y_fecha_limite() {
        let texto = explicacion_iva(&resumen(), fecha("2026-12-10"));
        assert!(texto.contains("13.00"), "{texto}");
        assert!(texto.contains("6.50"), "{texto}");
        assert!(texto.contains("10/12/2026"), "{texto}");
    }

    #[test]
    fn explicacion_pago_a_cuenta_incluye_montos_y_fecha_limite() {
        let texto = explicacion_pago_a_cuenta(&resumen(), fecha("2026-12-10"));
        assert!(texto.contains("1.75"), "{texto}");
        assert!(texto.contains("100.00"), "{texto}");
        assert!(texto.contains("10/12/2026"), "{texto}");
    }

    #[test]
    fn explicacion_renta_incluye_montos_y_fecha_limite() {
        let tramo = Tramo {
            desde: dec!(4064.01),
            hasta: Some(dec!(9142.86)),
            tasa: dec!(0.10),
            cuota_fija: dec!(212.12),
            exceso_sobre: dec!(4064.00),
        };
        let texto = explicacion_renta_natural(
            dec!(6000.00),
            Some(&tramo),
            dec!(405.72),
            dec!(100.00),
            dec!(305.72),
            fecha("2027-04-30"),
        );
        assert!(texto.contains("405.72"), "{texto}");
        assert!(texto.contains("305.72"), "{texto}");
        assert!(texto.contains("30/04/2027"), "{texto}");
    }

    #[test]
    fn explicacion_multa_incluye_montos_y_fecha_limite() {
        let texto = explicacion_multa(dec!(30.00), 3, fecha("2026-12-10"));
        assert!(texto.contains("30.00"), "{texto}");
        assert!(texto.contains("10/12/2026"), "{texto}");
    }
}
