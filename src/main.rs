use dotenv::dotenv;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod services;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenv().ok();

    // Initialize logging
    env_logger::init();

    // Create a shared instance of the Database wrapped in an Arc and Mutex
    let db = Arc::new(Mutex::new(services::db::Database::new().await));

    // Build the client-product matrix
    let matrix = {
        let mut db = db.lock().await;
        db.build_client_product_matrix().await?
    };

    Ok(())
}
