use super::mssql::SqlServerDatabase;
use crate::models::db::{Database, DatabaseTrait};
use crate::services::firebird::FirebirdDatabase;
use crate::services::training::find_best_als_model;
use crate::MODEL_SERVER;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use tokio_cron_scheduler::{Job, JobScheduler};

pub async fn schedule_jobs(notify: Arc<Notify>) -> Result<(), Box<dyn std::error::Error>> {
    let sched = JobScheduler::new().await?;

    let job_notify = notify.clone();
    let job = Job::new_async("at 12:00 am", move |_uuid, _l| {
        let job_notify = job_notify.clone();
        Box::pin(async move {
            println!("Executing model training and update job");
            let db_type = std::env::var("DB_TYPE").expect("DB_TYPE is not set in the environment");

            let db: Arc<Mutex<dyn DatabaseTrait + Send + Sync>> = match db_type.as_str() {
                // TODO Add support for Firebird
                "firebird" => Arc::new(Mutex::new(FirebirdDatabase::new())),
                "sqlserver" => Arc::new(Mutex::new(SqlServerDatabase::new().await)),
                _ => {
                    eprintln!("Unsupported DB_TYPE: '{}'", db_type);
                    return;
                }
            };
            let matrix = match Database::new(db).build_matrix().await {
                Ok(matrix) => matrix,
                Err(e) => {
                    eprintln!("Failed to build matrix: {}", e);
                    return;
                }
            };

            find_best_als_model(matrix, job_notify).await;
            MODEL_SERVER.initialize();
            println!("Model training and update job executed");
        })
    })?;

    sched.add(job).await?;
    sched.start().await?;

    Ok(())
}
