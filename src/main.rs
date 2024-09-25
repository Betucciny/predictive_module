use models::db::{Database, DatabaseTrait};
use services::mssql::SqlServerDatabase;
use std::sync::{Arc, Mutex};

pub mod models;
pub mod services;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    env_logger::init();

    let db_type = std::env::var("DB_TYPE").expect("DB_TYPE is not set in the environment");

    let db: Arc<Mutex<dyn DatabaseTrait + Send + Sync>> = match db_type.as_str() {
        // TODO Add support for Firebird
        "firebird" => Arc::new(Mutex::new(SqlServerDatabase::new().await)),
        "sqlserver" => Arc::new(Mutex::new(SqlServerDatabase::new().await)),
        _ => return Err(format!("Unsupported DB_TYPE: '{}'", db_type).into()),
    };

    let mut database = Database::new(db);
    database.build_matrix().await?;

    Ok(())
}
