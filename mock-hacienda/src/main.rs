use axum::{
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::net::SocketAddr;
use tracing::info;
use uuid::Uuid;

#[derive(Deserialize)]
struct ValidacionReq {
    factura_id: String,
    numero_control_simulado: String,
}

#[derive(Serialize)]
struct ValidacionRes {
    valido: bool,
    codigo_generacion: String,
    sello_recepcion: String,
    mensaje: String,
    simulado: bool,
}

async fn validar_dte(Json(payload): Json<ValidacionReq>) -> Json<ValidacionRes> {
    info!(
        "Recibida solicitud de validación simulada para factura: {}",
        payload.factura_id
    );

    // Simulador: valida y genera sellos mock de Hacienda El Salvador (DTE)
    Json(ValidacionRes {
        valido: true,
        codigo_generacion: format!("MOCK-DTE-{}", Uuid::new_v4()),
        sello_recepcion: format!("SELLO-SIMULADO-{}", Uuid::new_v4()),
        mensaje: "Documento Tributario Electrónico Validado (SIMULACIÓN)".into(),
        simulado: true,
    })
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    info!("Iniciando Simulador del Ministerio de Hacienda (El Salvador Mock)...");

    let app = Router::new()
        .route("/salud", get(|| async { Json(json!({"estado": "SIMULADOR_OK"})) }))
        .route("/validar-dte", post(validar_dte));

    let addr: SocketAddr = "0.0.0.0:8081".parse().unwrap();
    info!("Simulador Hacienda escuchando en http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
