use env_logger;
use models::db::{Database, DatabaseTrait};
use services::cronjobs::schedule_jobs;
use services::firebird::FirebirdDatabase;
use services::modelserver::ModelServer;
use services::mssql::SqlServerDatabase;
use services::training::find_best_als_model;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::{Mutex, Notify};

pub mod handlers;
pub mod models;
pub mod services;

lazy_static::lazy_static! {
    pub static ref MODEL_SERVER: Arc<ModelServer> = Arc::new(ModelServer::new("hyperparameters.json", Arc::new(Notify::new())));
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    env_logger::init();

    let db_type = std::env::var("DB_TYPE").expect("DB_TYPE is not set in the environment");

    let db: Arc<Mutex<dyn DatabaseTrait + Send + Sync>> = match db_type.as_str() {
        // TODO Add support for Firebird
        "sqlserver" => Arc::new(Mutex::new(SqlServerDatabase::new().await)),
        "firebird" => Arc::new(Mutex::new(FirebirdDatabase::new())),
        _ => return Err(format!("Unsupported DB_TYPE: '{}'", db_type).into()),
    };

    // Create a Notify instance for cancellation
    let notify = Arc::new(Notify::new());

    // Schedule jobs
    let job_handle = {
        let notify = notify.clone();
        tokio::spawn(async move {
            if let Err(e) = schedule_jobs(notify).await {
                eprintln!("Failed to schedule jobs: {}", e);
            }
        })
    };

    let matrix = Database::new(db).build_matrix().await.unwrap();

    // Spawn the find_best_als_model task
    let find_model_handle = {
        let notify = notify.clone();
        tokio::spawn(async move { find_best_als_model(matrix, notify).await })
    };

    // Initialize the MODEL_SERVER with the notify instance
    MODEL_SERVER.initialize();

    // Create the Warp filters
    let recommendation_routes = handlers::recommendations::recommendation_handler();

    // Start the Warp server
    let (addr, server) = warp::serve(recommendation_routes).bind_with_graceful_shutdown(
        ([127, 0, 0, 1], 3030),
        async {
            signal::ctrl_c()
                .await
                .expect("Failed to listen for ctrl_c signal");
        },
    );

    println!("Server running on http://{}", addr);

    tokio::select! {
        _ = server => {
            println!("Server has shut down");
        }
        _ = signal::ctrl_c() => {
            println!("Received Ctrl+C, shutting down");
            notify.notify_waiters(); // Signal cancellation
        }
    }

    // Cancel the scheduled jobs
    job_handle.abort();

    // Wait for the find_best_als_model task to complete
    if let Err(e) = find_model_handle.await {
        eprintln!("Error waiting for find_best_als_model task: {}", e);
    }

    println!("Application has shut down");

    Ok(())
}
