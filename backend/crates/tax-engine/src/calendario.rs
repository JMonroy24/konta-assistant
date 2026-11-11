//! Calendario fiscal: fechas límite de presentación y pago, y generación de
//! obligaciones (IVA F07, pago a cuenta F14 y renta anual F11).

use chrono::{Datelike, NaiveDate, Weekday};
use domain::{DomainError, Periodo, ReglaFiscal, TipoObligacion};
use serde::{Deserialize, Serialize};

use crate::reglas::{entero_desde, seleccionar_regla};

/// Cuenta `dias` días hábiles desde el primer día del mes siguiente al período.
pub const MODO_DIAS_HABILES_MES_SIGUIENTE: &str = "dias_habiles_mes_siguiente";
/// Fecha fija `mes`/`dia` dentro del año del período.
pub const MODO_FECHA_FIJA_ANUAL: &str = "fecha_fija_anual";

pub const CODIGO_PLAZO_IVA_F07: &str = "plazo_iva_f07";
pub const CODIGO_PLAZO_PAGO_CUENTA_F14: &str = "plazo_pago_cuenta_f14";
pub const CODIGO_PLAZO_RENTA_ANUAL_F11: &str = "plazo_renta_anual_f11";

/// Obligación fiscal con su fecha límite calculada.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Obligacion {
    pub tipo: TipoObligacion,
    pub fecha_limite: NaiveDate,
}

/// Calcula la fecha límite de una obligación a partir de su regla de plazo.
///
/// Modos soportados:
/// - [`MODO_DIAS_HABILES_MES_SIGUIENTE`]: cuenta `dias` días hábiles (lunes a
///   viernes, excluyendo los `feriados` recibidos) desde el primer día del mes
///   siguiente al período.
/// - [`MODO_FECHA_FIJA_ANUAL`]: fecha `mes`/`dia` dentro del año del período.
pub fn fecha_limite(
    regla: &ReglaFiscal,
    periodo: &Periodo,
    feriados: &[NaiveDate],
) -> Result<NaiveDate, DomainError> {
    let modo = regla
        .parametros
        .get("modo")
        .and_then(|valor| valor.as_str())
        .ok_or_else(|| {
            DomainError::ValidacionError(format!(
                "La regla '{}' no define el parámetro 'modo'",
                regla.codigo
            ))
        })?;

    match modo {
        MODO_DIAS_HABILES_MES_SIGUIENTE => {
            let dias = entero_desde(regla, "dias")?;
            dias_habiles_mes_siguiente(periodo, dias, feriados)
        }
        MODO_FECHA_FIJA_ANUAL => {
            let mes = entero_desde(regla, "mes")?;
            let dia = entero_desde(regla, "dia")?;
            fecha_fija_anual(periodo, mes, dia)
        }
        otro => Err(DomainError::ValidacionError(format!(
            "Modo de plazo desconocido en '{}': '{}'",
            regla.codigo, otro
        ))),
    }
}

fn es_dia_habil(fecha: NaiveDate, feriados: &[NaiveDate]) -> bool {
    let fin_de_semana = matches!(fecha.weekday(), Weekday::Sat | Weekday::Sun);
    !fin_de_semana && !feriados.contains(&fecha)
}

fn dias_habiles_mes_siguiente(
    periodo: &Periodo,
    dias: u32,
    feriados: &[NaiveDate],
) -> Result<NaiveDate, DomainError> {
    if dias == 0 {
        return Err(DomainError::ValidacionError(
            "El número de días hábiles debe ser mayor que cero".into(),
        ));
    }

    let siguiente = periodo.siguiente();
    let mut fecha = NaiveDate::from_ymd_opt(siguiente.anio(), siguiente.mes(), 1)
        .ok_or_else(|| DomainError::FechaInvalida(siguiente.to_string()))?;

    let mut contados = 0u32;
    loop {
        if es_dia_habil(fecha, feriados) {
            contados += 1;
            if contados == dias {
                return Ok(fecha);
            }
        }
        fecha = fecha
            .succ_opt()
            .ok_or_else(|| DomainError::FechaInvalida(fecha.to_string()))?;
    }
}

fn fecha_fija_anual(periodo: &Periodo, mes: u32, dia: u32) -> Result<NaiveDate, DomainError> {
    NaiveDate::from_ymd_opt(periodo.anio(), mes, dia).ok_or_else(|| {
        DomainError::FechaInvalida(format!("{:04}-{:02}-{:02}", periodo.anio(), mes, dia))
    })
}

/// Genera las obligaciones fiscales de un período: IVA F07, pago a cuenta F14 y
/// renta anual F11.
///
/// La regla de plazo vigente se selecciona a partir del último día del período.
/// Las obligaciones cuya regla de plazo falte, esté fuera de vigencia o tenga
/// parámetros inválidos se omiten del resultado.
pub fn generar_obligaciones(
    periodo: &Periodo,
    reglas: &[ReglaFiscal],
    feriados: &[NaiveDate],
) -> Vec<Obligacion> {
    let Ok((_, fecha_referencia)) = periodo.rango_fechas() else {
        return Vec::new();
    };

    const PLAZOS: [(TipoObligacion, &str); 3] = [
        (TipoObligacion::IvaF07, CODIGO_PLAZO_IVA_F07),
        (TipoObligacion::PagoCuentaF14, CODIGO_PLAZO_PAGO_CUENTA_F14),
        (TipoObligacion::RentaF11, CODIGO_PLAZO_RENTA_ANUAL_F11),
    ];

    let mut obligaciones = Vec::new();
    for (tipo, codigo) in PLAZOS {
        let Ok(regla) = seleccionar_regla(reglas, codigo, fecha_referencia) else {
            continue;
        };
        let Ok(fecha) = fecha_limite(regla, periodo, feriados) else {
            continue;
        };
        obligaciones.push(Obligacion {
            tipo,
            fecha_limite: fecha,
        });
    }
    obligaciones
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fecha(texto: &str) -> NaiveDate {
        NaiveDate::parse_from_str(texto, "%Y-%m-%d").expect("fecha de prueba válida")
    }

    fn periodo(texto: &str) -> Periodo {
        Periodo::parse(texto).expect("período de prueba válido")
    }

    fn regla(codigo: &str, parametros: serde_json::Value) -> ReglaFiscal {
        ReglaFiscal {
            id: format!("regla:{codigo}:v1"),
            rev: None,
            tipo: "regla_fiscal".into(),
            codigo: codigo.into(),
            vigencia_desde: fecha("2020-01-01"),
            vigencia_hasta: None,
            fuente_normativa: "prueba".into(),
            version: 1,
            verificada: true,
            parametros,
        }
    }

    fn regla_dias_habiles(codigo: &str, dias: u32) -> ReglaFiscal {
        regla(
            codigo,
            json!({ "modo": MODO_DIAS_HABILES_MES_SIGUIENTE, "dias": dias }),
        )
    }

    #[test]
    fn mes_que_termina_en_fin_de_semana() {
        // Mayo de 2026 termina en domingo; el cómputo arranca el lunes 1 de junio.
        let regla = regla_dias_habiles(CODIGO_PLAZO_IVA_F07, 10);
        let limite = fecha_limite(&regla, &periodo("2026-05"), &[]).expect("cálculo válido");
        assert_eq!(limite, fecha("2026-06-12"));
    }

    #[test]
    fn feriado_intermedio_se_excluye() {
        let regla = regla_dias_habiles(CODIGO_PLAZO_IVA_F07, 10);
        let feriados = [fecha("2026-06-03")];
        let limite = fecha_limite(&regla, &periodo("2026-05"), &feriados).expect("cálculo válido");
        assert_eq!(limite, fecha("2026-06-15"));
    }

    #[test]
    fn diciembre_pasa_a_enero() {
        let regla = regla_dias_habiles(CODIGO_PLAZO_IVA_F07, 10);
        let limite = fecha_limite(&regla, &periodo("2026-12"), &[]).expect("cálculo válido");
        assert_eq!(limite, fecha("2027-01-14"));
        assert_eq!(limite.year(), 2027);
    }

    #[test]
    fn fecha_fija_anual_29_de_febrero_en_bisiesto() {
        let regla = regla(
            CODIGO_PLAZO_RENTA_ANUAL_F11,
            json!({ "modo": MODO_FECHA_FIJA_ANUAL, "mes": 2, "dia": 29 }),
        );
        let limite = fecha_limite(&regla, &periodo("2028-01"), &[]).expect("2028 es bisiesto");
        assert_eq!(limite, fecha("2028-02-29"));
    }

    #[test]
    fn fecha_fija_anual_29_de_febrero_en_no_bisiesto_es_error() {
        let regla = regla(
            CODIGO_PLAZO_RENTA_ANUAL_F11,
            json!({ "modo": MODO_FECHA_FIJA_ANUAL, "mes": 2, "dia": 29 }),
        );
        assert!(matches!(
            fecha_limite(&regla, &periodo("2027-01"), &[]),
            Err(DomainError::FechaInvalida(_))
        ));
    }

    #[test]
    fn modo_desconocido_es_error() {
        let regla = regla(CODIGO_PLAZO_IVA_F07, json!({ "modo": "otro" }));
        assert!(matches!(
            fecha_limite(&regla, &periodo("2026-05"), &[]),
            Err(DomainError::ValidacionError(_))
        ));
    }

    #[test]
    fn generar_obligaciones_devuelve_las_tres() {
        let reglas = vec![
            regla_dias_habiles(CODIGO_PLAZO_IVA_F07, 10),
            regla_dias_habiles(CODIGO_PLAZO_PAGO_CUENTA_F14, 10),
            regla(
                CODIGO_PLAZO_RENTA_ANUAL_F11,
                json!({ "modo": MODO_FECHA_FIJA_ANUAL, "mes": 4, "dia": 30 }),
            ),
        ];

        let obligaciones = generar_obligaciones(&periodo("2026-05"), &reglas, &[]);
        assert_eq!(obligaciones.len(), 3);
        assert_eq!(
            obligaciones[0],
            Obligacion {
                tipo: TipoObligacion::IvaF07,
                fecha_limite: fecha("2026-06-12"),
            }
        );
        assert_eq!(
            obligaciones[1],
            Obligacion {
                tipo: TipoObligacion::PagoCuentaF14,
                fecha_limite: fecha("2026-06-12"),
            }
        );
        assert_eq!(
            obligaciones[2],
            Obligacion {
                tipo: TipoObligacion::RentaF11,
                fecha_limite: fecha("2026-04-30"),
            }
        );
    }

    #[test]
    fn generar_obligaciones_omite_reglas_ausentes() {
        let reglas = vec![regla_dias_habiles(CODIGO_PLAZO_IVA_F07, 10)];
        let obligaciones = generar_obligaciones(&periodo("2026-05"), &reglas, &[]);
        assert_eq!(obligaciones.len(), 1);
        assert_eq!(obligaciones[0].tipo, TipoObligacion::IvaF07);
    }
}
