use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TipoOperacion {
    Venta,
    Compra,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operacion {
    #[serde(rename = "_id")]
    pub id: String,
    #[serde(rename = "_rev", skip_serializing_if = "Option::is_none")]
    pub rev: Option<String>,
    pub tipo: TipoOperacion,
    pub fecha: NaiveDate,
    pub descripcion: String,
    pub base: Decimal,
    pub iva: Decimal,
    pub total: Decimal,
    pub contraparte: String, // Cliente o Proveedor
    pub creado_por: String,
    pub origen: String, // "local" o "remoto"
    pub creado_en: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EstadoFactura {
    Pendiente,
    AceptadoSimulado,
    Rechazado,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Factura {
    #[serde(rename = "_id")]
    pub id: String,
    #[serde(rename = "_rev", skip_serializing_if = "Option::is_none")]
    pub rev: Option<String>,
    pub tipo: String, // "factura"
    pub numero_control_simulado: String,
    pub estado_validacion: EstadoFactura,
    pub referencia_venta_id: String,
    pub hora_emision: DateTime<Utc>,
    pub mensaje_simulador: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EstadoDeclaracion {
    Borrador,
    Presentada,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Declaracion {
    #[serde(rename = "_id")]
    pub id: String,
    #[serde(rename = "_rev", skip_serializing_if = "Option::is_none")]
    pub rev: Option<String>,
    pub tipo: String, // "declaracion"
    pub periodo: String, // YYYY-MM
    pub tipo_impuesto: String, // "IVA_F07", "RENTA_F11"
    pub montos_calculados: serde_json::Value,
    pub estado: EstadoDeclaracion,
    pub fecha_presentacion: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EstadoRecordatorio {
    Pendiente,
    Enviado,
    Cumplido,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recordatorio {
    #[serde(rename = "_id")]
    pub id: String,
    #[serde(rename = "_rev", skip_serializing_if = "Option::is_none")]
    pub rev: Option<String>,
    pub tipo: String, // "recordatorio"
    pub obligacion: String,
    pub fecha_limite: NaiveDate,
    pub estado: EstadoRecordatorio,
    pub negocio_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReglaFiscal {
    #[serde(rename = "_id")]
    pub id: String,
    #[serde(rename = "_rev", skip_serializing_if = "Option::is_none")]
    pub rev: Option<String>,
    pub tipo: String, // "regla_fiscal"
    pub codigo: String,
    pub vigencia_desde: NaiveDate,
    pub vigencia_hasta: Option<NaiveDate>,
    pub fuente_normativa: String,
    pub version: u32,
    pub parametros: serde_json::Value,
}
