use dotenv::dotenv;
use log::info;
use std::sync::Arc;
use tokio::sync::Mutex;

mod cron_jobs;
mod handlers;
mod services;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenv().ok();

    // Initialize logging
    env_logger::init();

    // Create shared instances for cache and ALS
    let cache = Arc::new(Mutex::new(services::cache::RecommendationCache::new()));
    let als = Arc::new(services::als::ALS::new());
    let db = Arc::new(services::db::Database::new().await);

    // Schedule the training job
    cron_jobs::training_job::schedule_training_job(cache.clone(), als.clone(), db.clone()).await?;

    // Start the HTTP server
    let routes = handlers::recommendations::routes(cache.clone());
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;

    Ok(())
}
