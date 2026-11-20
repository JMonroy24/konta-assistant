mod handlers;
mod telemetry;

use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use handlers::*;
use infrastructure::{CouchDbRepository, MockHaciendaClient};
use std::{net::SocketAddr, sync::Arc};
use telemetry::telemetry_middleware;
use tower_http::cors::CorsLayer;
use tracing::info;

fn build_couchdb_url(raw_url: &str, user: &str, pass: &str) -> String {
    let trimmed = raw_url.trim_end_matches('/');
    if trimmed.contains('@') || user.is_empty() {
        return trimmed.to_string();
    }
    if let Some(rest) = trimmed.strip_prefix("http://") {
        format!("http://{}:{}@{}", user, pass, rest)
    } else if let Some(rest) = trimmed.strip_prefix("https://") {
        format!("https://{}:{}@{}", user, pass, rest)
    } else {
        trimmed.to_string()
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,api=debug".into()),
        )
        .json()
        .init();

    info!("Iniciando Konta Assistant Backend (X1 - CouchDB Clean Architecture)...");

    let couch_raw = std::env::var("COUCHDB_LOCAL_URL")
        .unwrap_or_else(|_| "http://couchdb-local:5984".into());
    let couch_user = std::env::var("COUCHDB_USER").unwrap_or_else(|_| "admin".into());
    let couch_pass =
        std::env::var("COUCHDB_PASSWORD").unwrap_or_else(|_| "adminsecretpassword".into());
    let mock_hacienda_url = std::env::var("MOCK_HACIENDA_URL")
        .unwrap_or_else(|_| "http://mock-hacienda:8081".into());
    let negocio_id = std::env::var("COUCHDB_NEGOCIO_ID").unwrap_or_else(|_| "demo".into());

    let couch_url = build_couchdb_url(&couch_raw, &couch_user, &couch_pass);
    info!(
        "Conectando a CouchDB local ({}) en base negocio_{}",
        couch_raw, negocio_id
    );

    let state = AppState {
        couchdb: Arc::new(CouchDbRepository::new(couch_url)),
        hacienda: Arc::new(MockHaciendaClient::new(mock_hacienda_url)),
        negocio_id,
    };

    let app = Router::new()
        // Auth & Roles
        .route("/auth/login", post(login_handler))
        // Ventas y Compras
        .route("/ventas", post(crear_operacion_handler))
        .route("/compras", post(crear_operacion_handler))
        .route("/operaciones", get(listar_operaciones_handler))
        // Facturación Electrónica Simulada
        .route("/facturas", post(emitir_factura_handler))
        // Motor Fiscal
        .route("/impuestos/resumen", get(resumen_impuestos_handler))
        // Declaraciones
        .route("/declaraciones/borrador", post(borrador_declaracion_handler))
        // Recordatorios
        .route("/recordatorios", get(recordatorios_handler))
        // Sincronización
        .route("/sync/estado", get(sync_estado_handler))
        // Salud y Telemetría
        .route("/salud", get(salud_handler))
        .route("/metricas", get(metricas_handler))
        .layer(middleware::from_fn(telemetry_middleware))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    let addr: SocketAddr = format!("0.0.0.0:{}", port)
        .parse()
        .expect("Dirección de bind inválida");

    info!("Servidor escuchando en http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
