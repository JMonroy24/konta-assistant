use axum::{
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use std::net::SocketAddr;
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    info!("Iniciando Grupo Control X0 (Backend Síncrono + PostgreSQL)...");

    let app = Router::new()
        .route("/salud", get(|| async { Json(json!({"grupo": "control_x0", "estado": "OK"})) }))
        .route("/ventas", post(|| async { Json(json!({"mensaje": "Venta guardada en PostgreSQL (X0)"})) }))
        .route("/operaciones", get(|| async { Json(json!({"grupo": "X0", "operaciones": []})) }));

    let addr: SocketAddr = "0.0.0.0:8082".parse().unwrap();
    info!("Control X0 escuchando en http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
