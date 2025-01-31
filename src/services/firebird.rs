use crate::models::db::{ClientProductMatrix, DatabaseTrait};
use async_trait::async_trait;
use rsfbclient::{builder_pure_rust, Connection, Queryable};
use rsfbclient_rust::RustFbClient;
use std::collections::HashMap;
use std::env;

pub struct FirebirdDatabase {
    conn: Option<Connection<RustFbClient>>,
}

impl FirebirdDatabase {
    pub fn new() -> Self {
        dotenv::dotenv().ok();

        let host = env::var("DB_HOST").expect("DB_HOST is not set");
        let port = env::var("DB_PORT")
            .expect("DB_PORT is not set")
            .parse::<u16>()
            .unwrap();
        let username = env::var("DB_USERNAME").expect("DB_USERNAME is not set");
        let password = env::var("DB_PASSWORD").expect("DB_PASSWORD is not set");
        let database = env::var("DB_NAME").expect("DB_NAME is not set");

        // Use pure rust builder
        let conn = builder_pure_rust()
            .host(&host)
            .port(port)
            .user(&username)
            .pass(&password)
            .db_name(&database)
            .connect()
            .unwrap();

        FirebirdDatabase { conn: Some(conn) }
    }
}

#[async_trait]
impl DatabaseTrait for FirebirdDatabase {
    async fn build_client_product_matrix(
        &mut self,
    ) -> Result<ClientProductMatrix, Box<dyn std::error::Error>> {
        let table_inve = env::var("TABLE_INVE").expect("TABLE_INVE is not set");
        let table_fact = env::var("TABLE_FACT").expect("TABLE_FACT is not set");
        let table_par_fact = env::var("TABLE_PAR_FACT").expect("TABLE_PAR_FACT is not set");
        let table_client = env::var("TABLE_CLIENT").expect("TABLE_CLIENT is not set");

        let excluded_clients: Vec<String> = env::var("EXCLUDED_CLIENTS")
            .unwrap_or_default()
            .split(',')
            .map(|s| format!("'{}'", s.trim()))
            .collect();

        let excluded_clients_clause = if !excluded_clients.is_empty() {
            format!("AND F.CVE_CLPV NOT IN ({})", excluded_clients.join(", "))
        } else {
            String::new()
        };

        let sql = format!(
                    "SELECT F.CVE_CLPV AS CLIENT_ID, PF.CVE_ART AS PRODUCT_ID, SUM(PF.CANT) AS TOTAL_QUANTITY
                     FROM {} AS PF
                     INNER JOIN {} AS I ON PF.CVE_ART = I.CVE_ART
                     INNER JOIN {} AS C INNER JOIN {} AS F ON C.CLAVE = F.CVE_CLPV
                     ON PF.CVE_DOC = F.CVE_DOC WHERE F.STATUS <> 'C'
                     AND C.NOMBRE NOT LIKE '%PUBLICO EN GENERAL%' {}
                     GROUP BY F.CVE_CLPV, PF.CVE_ART;",
                    table_par_fact, table_inve, table_client, table_fact, excluded_clients_clause
                );

        let mut matrix: ClientProductMatrix = HashMap::new();
        let rows = self.conn.as_mut().unwrap().query_iter(&sql, ())?;

        for row in rows {
            let (client_id, product_id, total_quantity): (String, String, f64) = row?;
            matrix
                .entry(client_id)
                .or_insert_with(HashMap::new)
                .insert(product_id, total_quantity);
        }

        Ok(matrix)
    }

    async fn close(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(conn) = self.conn.take() {
            conn.close()?;
        }
        Ok(())
    }
}
