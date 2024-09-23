use dotenv::dotenv;
use sqlx::{MssqlPool, Pool};
use std::env;

pub struct Database {
    pool: MssqlPool,
}

impl Database {
    pub async fn new() -> Self {
        dotenv().ok();
        let database_url =
            env::var("DATABASE_URL").expect("DATABASE_URL environment variable is not set");

        let pool = MssqlPool::connect(&database_url)
            .await
            .expect("Failed to connect to SQL Server");

        Database { pool }
    }

    // Example query: Fetching users (or replace with your actual query)
    pub async fn get_users(&self) -> Result<Vec<User>, sqlx::Error> {
        let users = sqlx::query_as!(User, "SELECT id, name FROM Users") // Replace with your actual SQL query
            .fetch_all(&self.pool)
            .await?;

        Ok(users)
    }
}

// Define the User struct to represent user data
#[derive(Debug, sqlx::FromRow)]
pub struct User {
    pub id: i32,
    pub name: String,
}
