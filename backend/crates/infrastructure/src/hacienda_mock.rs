use application::HaciendaSimulatorPort;
use async_trait::async_trait;
use domain::{DomainError, Factura};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct MockHaciendaClient {
    client: Client,
    endpoint_url: String,
}

#[derive(Serialize)]
struct SimulacionRequest {
    factura_id: String,
    numero_control_simulado: String,
}

#[derive(Deserialize)]
struct SimulacionResponse {
    valido: bool,
    codigo_generacion: Option<String>,
    mensaje: String,
}

impl MockHaciendaClient {
    pub fn new(endpoint_url: String) -> Self {
        Self {
            client: Client::new(),
            endpoint_url,
        }
    }
}

#[async_trait]
impl HaciendaSimulatorPort for MockHaciendaClient {
    async fn validar_documento(&self, factura: &Factura) -> Result<Result<String, String>, DomainError> {
        let req_body = SimulacionRequest {
            factura_id: factura.id.clone(),
            numero_control_simulado: factura.numero_control_simulado.clone(),
        };

        let resp = self
            .client
            .post(format!("{}/validar-dte", self.endpoint_url))
            .json(&req_body)
            .send()
            .await
            .map_err(|e| DomainError::ValidacionError(e.to_string()))?;

        if !resp.status().is_success() {
            return Ok(Err(format!("Fallo simulador: código HTTP {}", resp.status())));
        }

        let body: SimulacionResponse = resp
            .json()
            .await
            .map_err(|e| DomainError::ValidacionError(e.to_string()))?;

        if body.valido {
            Ok(Ok(body.codigo_generacion.unwrap_or_else(|| "SIMULADO-OK".into())))
        } else {
            Ok(Err(body.mensaje))
        }
    }
}
