use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub type ClientProductMatrix = HashMap<String, HashMap<String, f64>>;

#[async_trait]
pub trait DatabaseTrait {
    async fn build_client_product_matrix(
        &mut self,
    ) -> Result<ClientProductMatrix, Box<dyn std::error::Error>>;
}

pub struct Database {
    backend: Arc<Mutex<dyn DatabaseTrait + Send + Sync>>,
}

impl Database {
    pub fn new(backend: Arc<Mutex<dyn DatabaseTrait + Send + Sync>>) -> Self {
        Database { backend }
    }

    pub async fn build_matrix(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut backend = self.backend.lock().unwrap(); // Lock the mutex to get mutable access
        backend.build_client_product_matrix().await?;
        Ok(())
    }
}
