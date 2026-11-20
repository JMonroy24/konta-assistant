use application::{HaciendaSimulatorPort, OperacionRepository};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use domain::{EstadoFactura, Factura, Operacion, Periodo};
use infrastructure::{CouchDbRepository, MockHaciendaClient};
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use tax_engine::{MotorFiscal, ParametrosFiscales};
use tracing::{error, info};

#[derive(Clone)]
pub struct AppState {
    pub couchdb: Arc<CouchDbRepository>,
    pub hacienda: Arc<MockHaciendaClient>,
    pub negocio_id: String,
}

#[derive(Deserialize)]
pub struct LoginPayload {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct OperacionesQuery {
    pub mes: Option<String>,
}

pub async fn login_handler(Json(payload): Json<LoginPayload>) -> impl IntoResponse {
    if payload.username.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Usuario requerido"})),
        );
    }
    (
        StatusCode::OK,
        Json(json!({
            "token": "simulated_jwt_token",
            "username": payload.username,
            "rol": "administrador",
            "negocio_id": "negocio_demo"
        })),
    )
}

pub async fn crear_operacion_handler(
    State(state): State<AppState>,
    Json(operacion): Json<Operacion>,
) -> impl IntoResponse {
    let tasa_iva = Decimal::new(13, 2);
    if let Err(e) = operacion.validar(tasa_iva) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        );
    }

    match state
        .couchdb
        .guardar_operacion(&state.negocio_id, operacion)
        .await
    {
        Ok(guardada) => {
            info!("Operación {} guardada en CouchDB", guardada.id);
            (
                StatusCode::CREATED,
                Json(json!({
                    "mensaje": "Operación registrada con éxito",
                    "data": guardada
                })),
            )
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("409") {
                return (
                    StatusCode::CONFLICT,
                    Json(json!({ "error": "La operación ya existe en CouchDB" })),
                );
            }
            error!("Error guardando operación en CouchDB: {}", msg);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": msg })),
            )
        }
    }
}

pub async fn listar_operaciones_handler(
    State(state): State<AppState>,
    Query(params): Query<OperacionesQuery>,
) -> impl IntoResponse {
    let mes = params
        .mes
        .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m").to_string());

    match state.couchdb.listar_por_periodo(&state.negocio_id, &mes).await {
        Ok(operaciones) => (
            StatusCode::OK,
            Json(json!({
                "periodo": mes,
                "total": operaciones.len(),
                "operaciones": operaciones
            })),
        ),
        Err(e) => {
            error!("Error listando operaciones en CouchDB: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": e.to_string(),
                    "periodo": mes,
                    "total": 0,
                    "operaciones": []
                })),
            )
        }
    }
}

pub async fn emitir_factura_handler(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let id = payload["id"]
        .as_str()
        .or_else(|| payload["_id"].as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("factura:{}", uuid::Uuid::new_v4()));
    let numero_control = payload["numero_control_simulado"]
        .as_str()
        .unwrap_or("DTE-01-SIM-00001")
        .to_string();
    let referencia_venta_id = payload["referencia_venta_id"]
        .as_str()
        .unwrap_or("venta_demo")
        .to_string();

    let factura = Factura {
        id,
        rev: None,
        tipo: "factura".into(),
        numero_control_simulado: numero_control,
        estado_validacion: EstadoFactura::Pendiente,
        referencia_venta_id,
        hora_emision: chrono::Utc::now(),
        mensaje_simulador: None,
    };

    match state.hacienda.validar_documento(&factura).await {
        Ok(Ok(codigo)) => (
            StatusCode::CREATED,
            Json(json!({
                "estado": "aceptado_simulado",
                "codigo_generacion": codigo,
                "factura": factura
            })),
        ),
        Ok(Err(motivo)) => (
            StatusCode::ACCEPTED,
            Json(json!({
                "estado": "rechazado",
                "mensaje": motivo,
                "factura": factura
            })),
        ),
        Err(e) => (
            StatusCode::ACCEPTED,
            Json(json!({
                "estado": "pendiente_simulacion",
                "mensaje": format!("Simulador no disponible: {}", e),
                "factura": factura
            })),
        ),
    }
}

pub async fn resumen_impuestos_handler(
    State(state): State<AppState>,
    Query(params): Query<OperacionesQuery>,
) -> impl IntoResponse {
    let mes = params
        .mes
        .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m").to_string());

    let periodo = match Periodo::parse(&mes) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": e.to_string() })),
            );
        }
    };

    let operaciones = match state.couchdb.listar_por_periodo(&state.negocio_id, &mes).await {
        Ok(ops) => ops,
        Err(e) => {
            error!("Error consultando operaciones para resumen: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            );
        }
    };

    let params_fiscales = ParametrosFiscales {
        tasa_iva: Decimal::new(13, 2),
        tasa_pago_a_cuenta: Decimal::new(175, 4),
    };

    match MotorFiscal::calcular_impuestos(periodo, &operaciones, &params_fiscales, None) {
        Ok(resumen) => (
            StatusCode::OK,
            Json(json!({
                "periodo": mes,
                "ingresos_gravados": resumen.ingresos_gravados.to_string(),
                "debito_iva": resumen.debito.to_string(),
                "credito_iva": resumen.credito.to_string(),
                "iva_a_pagar": resumen.iva_a_pagar.to_string(),
                "remanente": resumen.remanente.to_string(),
                "pago_a_cuenta": resumen.pago_a_cuenta.to_string(),
                "estado": "calculado"
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        ),
    }
}

pub async fn borrador_declaracion_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "tipo": "declaracion",
            "estado": "borrador",
            "periodo": chrono::Utc::now().format("%Y-%m").to_string()
        })),
    )
}

pub async fn recordatorios_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "recordatorios": []
        })),
    )
}

pub async fn sync_estado_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "pendientes": 0,
            "ultimo_checkpoint": null,
            "conflictos": []
        })),
    )
}

pub async fn salud_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "estado": "OK",
            "timestamp": chrono::Utc::now().to_rfc3339()
        })),
    )
}

pub async fn metricas_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "throughput_rps": 0.0,
            "latencia_p50_ms": 0.0,
            "conflictos_detectados": 0
        })),
    )
}
