mod handlers;
mod telemetry;

use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use handlers::*;
use std::net::SocketAddr;
use telemetry::telemetry_middleware;
use tower_http::cors::CorsLayer;
use tracing::info;

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
        .layer(CorsLayer::permissive());

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    let addr: SocketAddr = format!("0.0.0.0:{}", port)
        .parse()
        .expect("Dirección de bind inválida");

    info!("Servidor escuchando en http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
