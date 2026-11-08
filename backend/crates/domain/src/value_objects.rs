use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumenImpuestos {
    pub periodo: String,
    pub debito_iva: rust_decimal::Decimal,
    pub credito_iva: rust_decimal::Decimal,
    pub iva_a_pagar: rust_decimal::Decimal,
    pub iva_remanente: rust_decimal::Decimal,
    pub pago_a_cuenta: rust_decimal::Decimal,
    pub fecha_limite_pago: chrono::NaiveDate,
    pub explicacion_calculo: String,
}
