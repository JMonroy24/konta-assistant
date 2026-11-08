use application::OperacionRepository;
use async_trait::async_trait;
use domain::{DomainError, Operacion};
use reqwest::Client;
use serde_json::json;

#[derive(Clone)]
pub struct CouchDbRepository {
    client: Client,
    base_url: String,
}

impl CouchDbRepository {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }

    fn db_url(&self, negocio_id: &str) -> String {
        format!("{}/negocio_{}", self.base_url, negocio_id)
    }
}

#[async_trait]
impl OperacionRepository for CouchDbRepository {
    async fn guardar_operacion(
        &self,
        negocio_id: &str,
        operacion: Operacion,
    ) -> Result<Operacion, DomainError> {
        let url = format!("{}/{}", self.db_url(negocio_id), operacion.id);
        let resp = self
            .client
            .put(&url)
            .json(&operacion)
            .send()
            .await
            .map_err(|e| DomainError::ValidacionError(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(DomainError::ValidacionError(format!(
                "Error guardando documento en CouchDB: {}",
                resp.status()
            )));
        }

        Ok(operacion)
    }

    async fn obtener_operacion(
        &self,
        negocio_id: &str,
        id: &str,
    ) -> Result<Option<Operacion>, DomainError> {
        let url = format!("{}/{}", self.db_url(negocio_id), id);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| DomainError::ValidacionError(e.to_string()))?;

        if resp.status().as_u16() == 404 {
            return Ok(None);
        }

        let operacion = resp
            .json::<Operacion>()
            .await
            .map_err(|e| DomainError::ValidacionError(e.to_string()))?;

        Ok(Some(operacion))
    }

    async fn listar_por_periodo(
        &self,
        negocio_id: &str,
        anio_mes: &str,
    ) -> Result<Vec<Operacion>, DomainError> {
        let url = format!("{}/_find", self.db_url(negocio_id));
        // Mango Query: busca documentos cuyo tipo sea venta o compra y su fecha inicie con YYYY-MM
        let query = json!({
            "selector": {
                "tipo": { "$in": ["venta", "compra"] },
                "fecha": {
                    "$gte": format!("{}-01", anio_mes),
                    "$lte": format!("{}-31", anio_mes)
                }
            },
            "limit": 1000
        });

        let resp = self
            .client
            .post(&url)
            .json(&query)
            .send()
            .await
            .map_err(|e| DomainError::ValidacionError(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(DomainError::ValidacionError(format!(
                "Error ejecutando Mango query: {}",
                resp.status()
            )));
        }

        let result: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| DomainError::ValidacionError(e.to_string()))?;

        let docs = result["docs"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        let mut operaciones = Vec::new();
        for doc in docs {
            if let Ok(op) = serde_json::from_value::<Operacion>(doc) {
                operaciones.push(op);
            }
        }

        Ok(operaciones)
    }
}
