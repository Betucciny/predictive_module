use crate::database::DatabaseTrait;
use std::sync::Arc;

pub struct Database {
    backend: Arc<dyn DatabaseTrait + Send + Sync>,
}

impl Database {
    pub fn new(backend: Arc<dyn DatabaseTrait + Send + Sync>) -> Self {
        Database { backend }
    }

    pub async fn build_matrix(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.backend.build_client_product_matrix().await?;
        Ok(())
    }
}
