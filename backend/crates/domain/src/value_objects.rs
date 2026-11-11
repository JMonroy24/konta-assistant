use chrono::NaiveDate;
use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};

use crate::errors::DomainError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RolUsuario {
    Administrador,
    Contador,
    Operador,
    Lectura,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usuario {
    pub id: String,
    pub username: String,
    pub rol: RolUsuario,
    pub negocio_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumenImpuestos {
    pub periodo: Periodo,
    pub ingresos_gravados: rust_decimal::Decimal,
    pub debito: rust_decimal::Decimal,
    pub credito: rust_decimal::Decimal,
    pub iva_a_pagar: rust_decimal::Decimal,
    pub remanente: rust_decimal::Decimal,
    /// Remanente a favor del período anterior aplicado contra el IVA a pagar.
    pub remanente_periodo_anterior: Option<rust_decimal::Decimal>,
    pub pago_a_cuenta: rust_decimal::Decimal,
}

/// Tipo de impuesto / formulario declarado ante Hacienda.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TipoImpuesto {
    IvaF07,
    PagoCuentaF14,
    RentaF11,
}

/// Obligación fiscal periódica que genera recordatorios.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TipoObligacion {
    IvaF07,
    PagoCuentaF14,
    RentaF11,
}

/// Obligación fiscal de un período, con su fecha límite ya calculada.
///
/// No contiene datos de negocio: es el resultado puro del calendario fiscal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObligacionFiscal {
    pub tipo: TipoObligacion,
    pub fecha_limite: NaiveDate,
}

/// Período fiscal mensual en formato `YYYY-MM`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Periodo {
    anio: i32,
    mes: u32,
}

impl Periodo {
    pub fn new(anio: i32, mes: u32) -> Result<Self, DomainError> {
        if !(1..=12).contains(&mes) {
            return Err(DomainError::FechaInvalida(format!("{anio:04}-{mes:02}")));
        }
        Ok(Self { anio, mes })
    }

    pub fn parse(texto: &str) -> Result<Self, DomainError> {
        let (anio_str, mes_str) = texto
            .split_once('-')
            .ok_or_else(|| DomainError::FechaInvalida(texto.to_string()))?;

        if anio_str.len() != 4 || mes_str.len() != 2 {
            return Err(DomainError::FechaInvalida(texto.to_string()));
        }

        let anio: i32 = anio_str
            .parse()
            .map_err(|_| DomainError::FechaInvalida(texto.to_string()))?;
        let mes: u32 = mes_str
            .parse()
            .map_err(|_| DomainError::FechaInvalida(texto.to_string()))?;

        Self::new(anio, mes)
    }

    pub fn anio(&self) -> i32 {
        self.anio
    }

    pub fn mes(&self) -> u32 {
        self.mes
    }

    pub fn siguiente(&self) -> Self {
        if self.mes == 12 {
            Self {
                anio: self.anio + 1,
                mes: 1,
            }
        } else {
            Self {
                anio: self.anio,
                mes: self.mes + 1,
            }
        }
    }

    /// Devuelve el primer y último día del período.
    pub fn rango_fechas(&self) -> Result<(NaiveDate, NaiveDate), DomainError> {
        let inicio = NaiveDate::from_ymd_opt(self.anio, self.mes, 1)
            .ok_or_else(|| DomainError::FechaInvalida(self.to_string()))?;
        let siguiente = self.siguiente();
        let inicio_siguiente = NaiveDate::from_ymd_opt(siguiente.anio, siguiente.mes, 1)
            .ok_or_else(|| DomainError::FechaInvalida(self.to_string()))?;
        let fin = inicio_siguiente
            .pred_opt()
            .ok_or_else(|| DomainError::FechaInvalida(self.to_string()))?;
        Ok((inicio, fin))
    }
}

impl std::fmt::Display for Periodo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04}-{:02}", self.anio, self.mes)
    }
}

impl Serialize for Periodo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Periodo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let texto = String::deserialize(deserializer)?;
        Periodo::parse(&texto).map_err(serde::de::Error::custom)
    }
}

/// Cantidad monetaria: newtype de `Decimal`, no negativa y redondeada a 2 decimales.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Dinero(Decimal);

impl Dinero {
    pub fn new(valor: Decimal) -> Result<Self, DomainError> {
        if valor < Decimal::ZERO {
            return Err(DomainError::ImporteInvalido);
        }
        Ok(Self(redondear(valor)))
    }

    pub fn cero() -> Self {
        Self(Decimal::ZERO)
    }

    pub fn valor(&self) -> Decimal {
        self.0
    }

    pub fn es_cero(&self) -> bool {
        self.0.is_zero()
    }

    pub fn sumar(&self, otro: Dinero) -> Result<Self, DomainError> {
        Self::new(self.0 + otro.0)
    }
}

fn redondear(valor: Decimal) -> Decimal {
    let mut redondeado = valor.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero);
    redondeado.rescale(2);
    redondeado
}

impl Serialize for Dinero {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&redondear(self.0).to_string())
    }
}

impl<'de> Deserialize<'de> for Dinero {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let texto = String::deserialize(deserializer)?;
        let valor: Decimal = texto.parse().map_err(serde::de::Error::custom)?;
        Dinero::new(valor).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn periodo_parse_valido() {
        let p = Periodo::parse("2026-11").unwrap();
        assert_eq!(p.anio(), 2026);
        assert_eq!(p.mes(), 11);
        assert_eq!(p.to_string(), "2026-11");
    }

    #[test]
    fn periodo_parse_sin_guion_es_invalido() {
        assert!(matches!(
            Periodo::parse("202611"),
            Err(DomainError::FechaInvalida(_))
        ));
    }

    #[test]
    fn periodo_parse_formato_incorrecto_es_invalido() {
        assert!(matches!(
            Periodo::parse("26-1"),
            Err(DomainError::FechaInvalida(_))
        ));
        assert!(matches!(
            Periodo::parse("2026-13"),
            Err(DomainError::FechaInvalida(_))
        ));
    }

    #[test]
    fn periodo_parse_no_numerico_es_invalido() {
        assert!(matches!(
            Periodo::parse("abcd-11"),
            Err(DomainError::FechaInvalida(_))
        ));
    }

    #[test]
    fn periodo_siguiente_normal() {
        assert_eq!(
            Periodo::new(2026, 11).unwrap().siguiente().to_string(),
            "2026-12"
        );
    }

    #[test]
    fn periodo_siguiente_cambia_de_anio() {
        assert_eq!(
            Periodo::new(2026, 12).unwrap().siguiente().to_string(),
            "2027-01"
        );
    }

    #[test]
    fn periodo_rango_fechas() {
        let (inicio, fin) = Periodo::new(2026, 2).unwrap().rango_fechas().unwrap();
        assert_eq!(inicio, NaiveDate::from_ymd_opt(2026, 2, 1).unwrap());
        assert_eq!(fin, NaiveDate::from_ymd_opt(2026, 2, 28).unwrap());
    }

    #[test]
    fn periodo_serde_ida_y_vuelta() {
        let p = Periodo::parse("2026-07").unwrap();
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(json, "\"2026-07\"");
        let recuperado: Periodo = serde_json::from_str(&json).unwrap();
        assert_eq!(p, recuperado);
    }

    #[test]
    fn dinero_redondea_a_dos_decimales() {
        let d = Dinero::new(dec!(10.005)).unwrap();
        assert_eq!(d.valor(), dec!(10.01));
    }

    #[test]
    fn dinero_no_acepta_negativo() {
        assert!(matches!(
            Dinero::new(dec!(-1.00)),
            Err(DomainError::ImporteInvalido)
        ));
    }

    #[test]
    fn dinero_serializa_como_string() {
        let d = Dinero::new(dec!(12.5)).unwrap();
        let json = serde_json::to_string(&d).unwrap();
        assert_eq!(json, "\"12.50\"");
    }

    #[test]
    fn dinero_deserializa_desde_string() {
        let d: Dinero = serde_json::from_str("\"99.99\"").unwrap();
        assert_eq!(d.valor(), dec!(99.99));
    }

    #[test]
    fn tipo_impuesto_serde_snake_case() {
        assert_eq!(
            serde_json::to_string(&TipoImpuesto::IvaF07).unwrap(),
            "\"iva_f07\""
        );
        assert_eq!(
            serde_json::to_string(&TipoImpuesto::PagoCuentaF14).unwrap(),
            "\"pago_cuenta_f14\""
        );
        assert_eq!(
            serde_json::to_string(&TipoImpuesto::RentaF11).unwrap(),
            "\"renta_f11\""
        );
    }

    #[test]
    fn tipo_obligacion_serde_snake_case() {
        assert_eq!(
            serde_json::to_string(&TipoObligacion::IvaF07).unwrap(),
            "\"iva_f07\""
        );
        let o: TipoObligacion = serde_json::from_str("\"pago_cuenta_f14\"").unwrap();
        assert_eq!(o, TipoObligacion::PagoCuentaF14);
    }
}
