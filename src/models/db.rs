use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type ClientProductMatrix = HashMap<String, HashMap<String, f64>>;

#[async_trait]
pub trait DatabaseTrait {
    async fn build_client_product_matrix(
        &mut self,
    ) -> Result<ClientProductMatrix, Box<dyn std::error::Error>>;
    async fn close(&mut self) -> Result<(), Box<dyn std::error::Error>>;
}

pub struct Database {
    backend: Arc<Mutex<dyn DatabaseTrait + Send + Sync>>,
}

#[derive(Debug)]
pub enum DatabaseError {
    ConnectionError(String),
    CloseError(String),
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseError::ConnectionError(msg) => write!(f, "Connection Error: {}", msg),
            DatabaseError::CloseError(msg) => write!(f, "Close Error: {}", msg),
        }
    }
}

impl std::error::Error for DatabaseError {}

impl Database {
    pub fn new(backend: Arc<Mutex<dyn DatabaseTrait + Send + Sync>>) -> Self {
        Database { backend }
    }

    pub async fn build_matrix(&mut self) -> Result<ClientProductMatrix, DatabaseError> {
        let mut backend = self.backend.lock().await;
        let matrix = backend
            .build_client_product_matrix()
            .await
            .map_err(|e| DatabaseError::ConnectionError(format!("Error building matrix: {}", e)))?;
        backend
            .close()
            .await
            .map_err(|e| DatabaseError::CloseError(format!("Error closing connection: {}", e)))?;
        Ok(matrix)
    }
}
