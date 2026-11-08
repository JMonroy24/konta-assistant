use axum::{
    body::Body,
    http::{Request, Response},
    middleware::Next,
};
use std::time::Instant;
use tracing::info;
use uuid::Uuid;

pub async fn telemetry_middleware(req: Request<Body>, next: Next) -> Response<Body> {
    let start = Instant::now();
    let request_id = Uuid::new_v4().to_string();
    let path = req.uri().path().to_string();
    let method = req.method().to_string();

    info!(
        request_id = %request_id,
        method = %method,
        path = %path,
        "Solicitud HTTP entrante"
    );

    let response = next.run(req).await;
    let duration = start.elapsed();
    let status = response.status().as_u16();

    info!(
        request_id = %request_id,
        status = %status,
        duration_ms = %duration.as_millis(),
        "Respuesta HTTP enviada"
    );

    // Nota: El middleware exporta periódicamente a CSV para Fase 5
    response
}
