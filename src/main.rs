use env_logger;
use handlers::recommendations::global_handler;
use services::cronjobs::schedule_jobs;
use services::modelserver::ModelServer;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::{Mutex, Notify};

pub mod handlers;
pub mod models;
pub mod services;

lazy_static::lazy_static! {
    pub static ref MODEL_SERVER: Arc<Mutex<ModelServer>> = ModelServer::new("./data/hyperparameters.json");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    env_logger::init();

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

    {
        let mut model_server_lock = MODEL_SERVER.lock().await;
        model_server_lock.initialize(notify.clone()).await.unwrap();
    }

    // Create the Warp filters
    let routes = global_handler();

    // Start the Warp server
    let (addr, server) =
        warp::serve(routes).bind_with_graceful_shutdown(([0, 0, 0, 0], 3030), async {
            signal::ctrl_c()
                .await
                .expect("Failed to listen for ctrl_c signal");
        });

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

    println!("Application has shut down");

    Ok(())
}
