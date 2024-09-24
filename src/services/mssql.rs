use futures::stream::TryStreamExt;
use std::collections::HashMap;
use std::env;
use tiberius::{AuthMethod, Client, Config};
use tokio::net::TcpStream;
use tokio_util::compat::TokioAsyncWriteCompatExt; // Required for async compatibility with `tiberius` // Use TryStreamExt for try_next()

// Type alias for a client-product matrix
pub type ClientProductMatrix = HashMap<String, HashMap<String, f64>>;

pub struct Database {
    client: Client<tokio_util::compat::Compat<TcpStream>>,
}

impl Database {
    // Create a new instance of the Database and connect to the SQL Server
    pub async fn new() -> Self {
        dotenv::dotenv().ok(); // Load environment variables from .env file

        // Create a new SQL Server configuration
        let mut config = Config::new();
        config.host(env::var("DB_HOST").expect("DB_HOST is not set"));
        config.port(
            env::var("DB_PORT")
                .expect("DB_PORT is not set")
                .parse::<u16>()
                .unwrap(),
        );
        config.authentication(AuthMethod::sql_server(
            env::var("DB_USERNAME").expect("DB_USERNAME is not set"),
            env::var("DB_PASSWORD").expect("DB_PASSWORD is not set"),
        ));
        config.database(env::var("DB_NAME").expect("DB_NAME is not set"));

        // Accept self-signed certificates
        config.trust_cert(); // This line allows self-signed certificates

        // Establish the connection
        let tcp = TcpStream::connect(config.get_addr()).await.unwrap();
        tcp.set_nodelay(true).unwrap();
        let client = Client::connect(config, tcp.compat_write()).await.unwrap();

        Database { client }
    }

    // Build the client-product matrix for ALS training
    pub async fn build_client_product_matrix(
        &mut self,
    ) -> Result<ClientProductMatrix, tiberius::error::Error> {
        let table_inve = env::var("TABLE_INVE").expect("TABLE_INVE is not set");
        let table_fact = env::var("TABLE_FACT").expect("TABLE_FACT is not set");
        let table_par_fact = env::var("TABLE_PAR_FACT").expect("TABLE_PAR_FACT is not set");
        let table_client = env::var("TABLE_CLIENT").expect("TABLE_CLIENT is not set");

        // Get the excluded clients from the environment variable and convert to SQL-compatible format
        let excluded_clients_env = env::var("EXCLUDED_CLIENTS").unwrap_or_else(|_| "".to_string());
        let excluded_clients: Vec<String> = excluded_clients_env
            .split(',')
            .map(|s| format!("'{}'", s))
            .collect();
        let excluded_clients_sql = if !excluded_clients.is_empty() {
            format!("AND F.CVE_CLPV NOT IN ({})", excluded_clients.join(","))
        } else {
            String::new()
        };

        let query = format!(
            r#"
            SELECT F.CVE_CLPV AS CLIENT_ID,
                   PF.CVE_ART AS PRODUCT_ID,
                   SUM(PF.CANT) AS TOTAL_QUANTITY
            FROM dbo.{} AS PF
                 INNER JOIN dbo.{} AS I ON PF.CVE_ART = I.CVE_ART
                 INNER JOIN dbo.{} AS C
                 INNER JOIN dbo.{} AS F ON C.CLAVE = F.CVE_CLPV
                            ON PF.CVE_DOC = F.CVE_DOC
            WHERE F.STATUS <> 'C'
              AND C.NOMBRE NOT LIKE '%PUBLICO EN GENERAL%'
              {}
            GROUP BY F.CVE_CLPV, PF.CVE_ART;
            "#,
            table_par_fact, table_inve, table_client, table_fact, excluded_clients_sql
        );
        println!("Query: {}", query);

        // Execute the query
        let mut result = self.client.query(query, &[]).await?;
        let mut matrix: ClientProductMatrix = HashMap::new();

        // Process the query results and build the client-product matrix
        while let Some(item) = result.try_next().await? {
            if let Some(row) = item.into_row() {
                // Extracting values from the row
                let client_id: &str = row.get::<&str, _>(0).unwrap_or("unknown_client");
                let product_id: &str = row.get::<&str, _>(1).unwrap_or("unknown_product");
                let total_quantity: f64 = row.get::<f64, _>(2).unwrap_or(0.0);

                matrix
                    .entry(client_id.to_string())
                    .or_insert_with(HashMap::new)
                    .insert(product_id.to_string(), total_quantity);
            }
        }

        Ok(matrix)
    }
}
