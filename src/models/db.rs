use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
pub type ClientProductMatrix = HashMap<String, HashMap<String, f64>>;

#[derive(Serialize, Deserialize)]
pub struct ClientRow {
    pub id: String,
    pub name: String,
    pub email: String,
}

#[derive(Serialize, Deserialize)]
pub struct ClientPage {
    pub current_page: i64,
    pub total_pages: i64,
    pub clients: Vec<ClientRow>,
}

#[derive(Serialize, Deserialize)]
pub struct ProductRow {
    pub id: String,
    pub description: String,
    pub price: f64,
}

#[derive(Serialize, Deserialize)]
pub struct ProductPage {
    pub current_page: i64,
    pub total_pages: i64,
    pub products: Vec<ProductRow>,
}

#[async_trait]
pub trait DatabaseTrait {
    async fn build_client_product_matrix(
        &mut self,
    ) -> Result<ClientProductMatrix, Box<dyn std::error::Error>>;
    async fn close(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    async fn get_clients(
        &mut self,
        search: String,
        page: i64,
    ) -> Result<ClientPage, Box<dyn std::error::Error>>;
    async fn get_products(
        &mut self,
        search: String,
        page: i64,
    ) -> Result<ProductPage, Box<dyn std::error::Error>>;

    async fn get_client_by_id(
        &mut self,
        id: String,
    ) -> Result<ClientRow, Box<dyn std::error::Error>>;
    async fn get_product_by_id(
        &mut self,
        id: String,
    ) -> Result<ProductRow, Box<dyn std::error::Error>>;
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
        Ok(matrix)
    }
    pub async fn get_clients(
        &mut self,
        search: String,
        page: i64,
    ) -> Result<ClientPage, DatabaseError> {
        let mut backend = self.backend.lock().await;
        backend
            .get_clients(search, page)
            .await
            .map_err(|e| DatabaseError::ConnectionError(format!("Error getting clients: {}", e)))
    }

    pub async fn get_products(
        &mut self,
        search: String,
        page: i64,
    ) -> Result<ProductPage, DatabaseError> {
        let mut backend = self.backend.lock().await;
        backend
            .get_products(search, page)
            .await
            .map_err(|e| DatabaseError::ConnectionError(format!("Error getting products: {}", e)))
    }

    pub async fn get_client_by_id(&mut self, id: String) -> Result<ClientRow, DatabaseError> {
        let mut backend = self.backend.lock().await;
        backend
            .get_client_by_id(id)
            .await
            .map_err(|e| DatabaseError::ConnectionError(format!("Error getting client: {}", e)))
    }

    pub async fn get_product_by_id(&mut self, id: String) -> Result<ProductRow, DatabaseError> {
        let mut backend = self.backend.lock().await;
        backend
            .get_product_by_id(id)
            .await
            .map_err(|e| DatabaseError::ConnectionError(format!("Error getting product: {}", e)))
    }

    pub async fn close(&mut self) -> Result<(), DatabaseError> {
        let mut backend = self.backend.lock().await;
        backend
            .close()
            .await
            .map_err(|e| DatabaseError::CloseError(format!("Error closing connection: {}", e)))
    }
}
