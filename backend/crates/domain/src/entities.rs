use crate::errors::DomainError;
use crate::value_objects::{TipoImpuesto, TipoObligacion};
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::{Decimal, RoundingStrategy};
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

impl Operacion {
    /// Valida los invariantes de la operación para la tasa de IVA vigente.
    pub fn validar(&self, tasa_iva: Decimal) -> Result<(), DomainError> {
        if self.descripcion.trim().is_empty() {
            return Err(DomainError::ValidacionError(
                "La descripción no puede estar vacía".into(),
            ));
        }

        if self.base < Decimal::ZERO || self.iva < Decimal::ZERO || self.total < Decimal::ZERO {
            return Err(DomainError::ImporteInvalido);
        }

        if self.total != self.base + self.iva {
            return Err(DomainError::ValidacionError(
                "El total debe ser igual a base + IVA".into(),
            ));
        }

        let iva_esperado = (self.base * tasa_iva)
            .round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero);
        if self.iva != iva_esperado {
            return Err(DomainError::ValidacionError(format!(
                "El IVA ({}) no corresponde a la tasa vigente ({})",
                self.iva, tasa_iva
            )));
        }

        Ok(())
    }
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
    pub tipo: String,    // "declaracion"
    pub periodo: String, // YYYY-MM
    pub tipo_impuesto: TipoImpuesto,
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
    pub obligacion: TipoObligacion,
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
    /// Indica si la regla fue verificada contra la normativa vigente. Mientras
    /// sea `false`, no debe usarse para estimar montos (p. ej. multas).
    #[serde(default)]
    pub verificada: bool,
    pub parametros: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EstadoConflicto {
    Pendiente,
    Resuelto,
}

/// Revisión (rama) que difiere de la versión ganadora.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RevisionDoc {
    #[serde(rename = "_rev")]
    pub rev: String,
    pub base: Decimal,
    pub iva: Decimal,
    pub total: Decimal,
}

/// Conflicto de replicación detectado entre revisiones de un mismo documento.
///
/// Solo describe el conflicto; nunca fusiona los importes. `detectado_en` queda
/// en `None` a propósito, para que la función de detección permanezca pura y el
/// instante lo fije quien persista el conflicto.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Conflicto {
    pub doc_id: String,
    pub revision_ganadora: String,
    pub revisiones_en_conflicto: Vec<RevisionDoc>,
    pub campos_en_conflicto: Vec<String>,
    pub estado: EstadoConflicto,
    pub detectado_en: Option<DateTime<Utc>>,
}

/// Detecta si alguna de las revisiones perdedoras difiere de la ganadora en
/// `base`, `iva` o `total`. Devuelve `None` si no hay diferencias.
///
/// Función pura: no muta las operaciones ni fusiona valores; únicamente
/// describe el conflicto para que un humano lo resuelva.
pub fn detectar_conflicto_importes(
    ganadora: &Operacion,
    perdedoras: &[Operacion],
) -> Option<Conflicto> {
    let mut difiere_base = false;
    let mut difiere_iva = false;
    let mut difiere_total = false;
    let mut revisiones_en_conflicto = Vec::new();

    for perdedora in perdedoras {
        let dif_base = perdedora.base != ganadora.base;
        let dif_iva = perdedora.iva != ganadora.iva;
        let dif_total = perdedora.total != ganadora.total;

        if !(dif_base || dif_iva || dif_total) {
            continue;
        }

        difiere_base |= dif_base;
        difiere_iva |= dif_iva;
        difiere_total |= dif_total;

        revisiones_en_conflicto.push(RevisionDoc {
            rev: perdedora.rev.clone().unwrap_or_default(),
            base: perdedora.base,
            iva: perdedora.iva,
            total: perdedora.total,
        });
    }

    if revisiones_en_conflicto.is_empty() {
        return None;
    }

    let mut campos_en_conflicto = Vec::new();
    if difiere_base {
        campos_en_conflicto.push("base".to_string());
    }
    if difiere_iva {
        campos_en_conflicto.push("iva".to_string());
    }
    if difiere_total {
        campos_en_conflicto.push("total".to_string());
    }

    Some(Conflicto {
        doc_id: ganadora.id.clone(),
        revision_ganadora: ganadora.rev.clone().unwrap_or_default(),
        revisiones_en_conflicto,
        campos_en_conflicto,
        estado: EstadoConflicto::Pendiente,
        detectado_en: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn operacion(base: Decimal, iva: Decimal, total: Decimal, descripcion: &str) -> Operacion {
        Operacion {
            id: "op-1".into(),
            rev: None,
            tipo: TipoOperacion::Venta,
            fecha: NaiveDate::from_ymd_opt(2026, 11, 5).unwrap(),
            descripcion: descripcion.into(),
            base,
            iva,
            total,
            contraparte: "Cliente A".into(),
            creado_por: "usr1".into(),
            origen: "local".into(),
            creado_en: Utc::now(),
        }
    }

    #[test]
    fn operacion_valida_es_aceptada() {
        let op = operacion(dec!(100.00), dec!(13.00), dec!(113.00), "Venta servicio");
        assert!(op.validar(dec!(0.13)).is_ok());
    }

    #[test]
    fn operacion_total_inconsistente_es_rechazada() {
        let op = operacion(dec!(100.00), dec!(13.00), dec!(120.00), "Venta servicio");
        assert!(matches!(
            op.validar(dec!(0.13)),
            Err(DomainError::ValidacionError(_))
        ));
    }

    #[test]
    fn operacion_con_importe_negativo_es_rechazada() {
        let op = operacion(dec!(-100.00), dec!(13.00), dec!(-87.00), "Venta servicio");
        assert!(matches!(
            op.validar(dec!(0.13)),
            Err(DomainError::ImporteInvalido)
        ));
    }

    #[test]
    fn operacion_descripcion_vacia_es_rechazada() {
        let op = operacion(dec!(100.00), dec!(13.00), dec!(113.00), "   ");
        assert!(matches!(
            op.validar(dec!(0.13)),
            Err(DomainError::ValidacionError(_))
        ));
    }

    #[test]
    fn operacion_iva_no_corresponde_a_tasa_es_rechazada() {
        let op = operacion(dec!(100.00), dec!(5.00), dec!(105.00), "Venta servicio");
        assert!(matches!(
            op.validar(dec!(0.13)),
            Err(DomainError::ValidacionError(_))
        ));
    }

    #[test]
    fn declaracion_serializa_tipo_impuesto_snake_case() {
        let declaracion = Declaracion {
            id: "d-1".into(),
            rev: None,
            tipo: "declaracion".into(),
            periodo: "2026-11".into(),
            tipo_impuesto: TipoImpuesto::PagoCuentaF14,
            montos_calculados: serde_json::json!({}),
            estado: EstadoDeclaracion::Borrador,
            fecha_presentacion: None,
        };
        let json = serde_json::to_value(&declaracion).unwrap();
        assert_eq!(json["tipo_impuesto"], "pago_cuenta_f14");
    }

    #[test]
    fn recordatorio_deserializa_obligacion_snake_case() {
        let json = serde_json::json!({
            "_id": "r-1",
            "tipo": "recordatorio",
            "obligacion": "iva_f07",
            "fecha_limite": "2026-12-10",
            "estado": "pendiente",
            "negocio_id": "negocio_001"
        });
        let recordatorio: Recordatorio = serde_json::from_value(json).unwrap();
        assert_eq!(recordatorio.obligacion, TipoObligacion::IvaF07);
    }

    fn con_revision(id: &str, rev: &str, base: Decimal, iva: Decimal, total: Decimal) -> Operacion {
        let mut op = operacion(base, iva, total, "Venta servicio");
        op.id = id.into();
        op.rev = Some(rev.into());
        op
    }

    #[test]
    fn conflicto_sin_diferencias_es_none() {
        let ganadora = con_revision("doc-1", "2-aaa", dec!(100.00), dec!(13.00), dec!(113.00));
        let identica = con_revision("doc-1", "1-bbb", dec!(100.00), dec!(13.00), dec!(113.00));
        assert!(detectar_conflicto_importes(&ganadora, &[identica]).is_none());
    }

    #[test]
    fn conflicto_sin_perdedoras_es_none() {
        let ganadora = con_revision("doc-1", "2-aaa", dec!(100.00), dec!(13.00), dec!(113.00));
        assert!(detectar_conflicto_importes(&ganadora, &[]).is_none());
    }

    #[test]
    fn conflicto_difiere_base() {
        let ganadora = con_revision("doc-1", "2-aaa", dec!(100.00), dec!(13.00), dec!(113.00));
        let perdedora = con_revision("doc-1", "1-bbb", dec!(200.00), dec!(13.00), dec!(113.00));

        let conflicto = detectar_conflicto_importes(&ganadora, &[perdedora]).unwrap();
        assert_eq!(conflicto.doc_id, "doc-1");
        assert_eq!(conflicto.revision_ganadora, "2-aaa");
        assert_eq!(conflicto.campos_en_conflicto, vec!["base".to_string()]);
        assert_eq!(conflicto.estado, EstadoConflicto::Pendiente);
        assert_eq!(conflicto.detectado_en, None);
        assert_eq!(conflicto.revisiones_en_conflicto.len(), 1);
        assert_eq!(conflicto.revisiones_en_conflicto[0].rev, "1-bbb");
        assert_eq!(conflicto.revisiones_en_conflicto[0].base, dec!(200.00));
    }

    #[test]
    fn conflicto_difiere_total() {
        let ganadora = con_revision("doc-1", "2-aaa", dec!(100.00), dec!(13.00), dec!(113.00));
        let perdedora = con_revision("doc-1", "1-bbb", dec!(100.00), dec!(13.00), dec!(120.00));

        let conflicto = detectar_conflicto_importes(&ganadora, &[perdedora]).unwrap();
        assert_eq!(conflicto.campos_en_conflicto, vec!["total".to_string()]);
        assert_eq!(conflicto.revisiones_en_conflicto[0].total, dec!(120.00));
    }

    #[test]
    fn conflicto_varias_ramas() {
        let ganadora = con_revision("doc-1", "3-aaa", dec!(100.00), dec!(13.00), dec!(113.00));
        let rama_total = con_revision("doc-1", "2-bbb", dec!(100.00), dec!(13.00), dec!(120.00));
        let rama_base = con_revision("doc-1", "1-ccc", dec!(200.00), dec!(13.00), dec!(113.00));

        let conflicto = detectar_conflicto_importes(&ganadora, &[rama_total, rama_base]).unwrap();
        assert_eq!(
            conflicto.campos_en_conflicto,
            vec!["base".to_string(), "total".to_string()]
        );
        assert_eq!(conflicto.revisiones_en_conflicto.len(), 2);
        // Nunca fusiona: cada rama conserva sus importes originales.
        assert_eq!(conflicto.revisiones_en_conflicto[0].rev, "2-bbb");
        assert_eq!(conflicto.revisiones_en_conflicto[0].total, dec!(120.00));
        assert_eq!(conflicto.revisiones_en_conflicto[1].rev, "1-ccc");
        assert_eq!(conflicto.revisiones_en_conflicto[1].base, dec!(200.00));
        // La ganadora permanece intacta.
        assert_eq!(ganadora.base, dec!(100.00));
        assert_eq!(ganadora.total, dec!(113.00));
    }
}
