use crate::models::db::{
    ClientPage, ClientProductMatrix, ClientRow, DatabaseTrait, ProductPage, ProductRow,
};
use async_trait::async_trait;
use futures::TryStreamExt;
use std::collections::HashMap;
use std::env;
use tiberius::{AuthMethod, Client, Config};
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

pub struct SqlServerDatabase {
    client: Option<Client<Compat<TcpStream>>>,
}

impl SqlServerDatabase {
    pub async fn new() -> Self {
        dotenv::dotenv().ok();

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
        config.trust_cert();

        let tcp = TcpStream::connect(config.get_addr()).await.unwrap();
        tcp.set_nodelay(true).unwrap();
        let client = Client::connect(config, tcp.compat_write()).await.unwrap();

        SqlServerDatabase {
            client: Some(client),
        }
    }
}

#[async_trait]
impl DatabaseTrait for SqlServerDatabase {
    async fn build_client_product_matrix(
        &mut self,
    ) -> Result<ClientProductMatrix, Box<dyn std::error::Error>> {
        // Your SQL Server specific code
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

        let query = format!(
                    "SELECT F.CVE_CLPV AS CLIENT_ID, PF.CVE_ART AS PRODUCT_ID, SUM(PF.CANT) AS TOTAL_QUANTITY
                     FROM dbo.{} AS PF
                     INNER JOIN dbo.{} AS I ON PF.CVE_ART = I.CVE_ART
                     INNER JOIN dbo.{} AS C INNER JOIN dbo.{} AS F ON C.CLAVE = F.CVE_CLPV
                     ON PF.CVE_DOC = F.CVE_DOC WHERE F.STATUS <> 'C'
                     AND C.NOMBRE NOT LIKE '%PUBLICO EN GENERAL%' {}
                     GROUP BY F.CVE_CLPV, PF.CVE_ART;",
                    table_par_fact, table_inve, table_client, table_fact, excluded_clients_clause
                );

        let mut result = self.client.as_mut().unwrap().query(query, &[]).await?;
        let mut matrix: ClientProductMatrix = HashMap::new();

        while let Some(item) = result.try_next().await? {
            if let Some(row) = item.into_row() {
                let client_id: String = row
                    .get::<&str, _>(0)
                    .unwrap_or("unknown_client")
                    .to_string();
                let product_id: String = row
                    .get::<&str, _>(1)
                    .unwrap_or("unknown_product")
                    .to_string();
                let total_quantity: f64 = row.get::<f64, _>(2).unwrap_or(0.0);
                matrix
                    .entry(client_id)
                    .or_insert_with(HashMap::new)
                    .insert(product_id, total_quantity);
            }
        }

        Ok(matrix)
    }

    async fn close(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(client) = self.client.take() {
            client.close().await?;
        }
        Ok(())
    }

    async fn get_clients(
        &mut self,
        search: String,
        page: i64,
    ) -> Result<ClientPage, Box<dyn std::error::Error>> {
        let table_client = env::var("TABLE_CLIENT").expect("TABLE_CLIENT is not set");
        let excluded_clients: Vec<String> = env::var("EXCLUDED_CLIENTS")
            .unwrap_or_default()
            .split(',')
            .map(|s| format!("'{}'", s.trim()))
            .collect();

        let excluded_clients_clause = if !excluded_clients.is_empty() {
            format!("AND CLAVE NOT IN ({})", excluded_clients.join(", "))
        } else {
            String::new()
        };

        let offset = (page - 1) * 10;

        let query1 = format!(
            "SELECT CLAVE as id, NOMBRE as name, EMAILPRED as email
             FROM dbo.{}
             WHERE NOMBRE LIKE '%{}%'
             AND NOMBRE NOT LIKE '%PUBLICO EN GENERAL%' {}
             ORDER BY CLAVE
             OFFSET {} ROWS
             FETCH NEXT 10 ROWS ONLY;",
            table_client, search, excluded_clients_clause, offset,
        );

        let query2 = format!(
            "SELECT (COUNT(*)-1)/10+1 as total_pages
             FROM dbo.{}
             WHERE NOMBRE LIKE '%{}%'
             AND NOMBRE NOT LIKE '%PUBLICO EN GENERAL%' {}",
            table_client, search, excluded_clients_clause,
        );

        let client = self.client.as_mut().unwrap();

        // Execute the first query and process the results
        let clients = {
            let mut result = client.query(query1, &[]).await?;
            let mut clients = Vec::new();
            while let Some(item) = result.try_next().await? {
                if let Some(row) = item.into_row() {
                    let id: String = row.get::<&str, _>(0).unwrap_or("unknown_id").to_string();
                    let name: String = row.get::<&str, _>(1).unwrap_or("unknown_name").to_string();
                    let email: String =
                        row.get::<&str, _>(2).unwrap_or("unknown_email").to_string();

                    clients.push(ClientRow { id, name, email });
                }
            }
            clients
        };
        let total_pages = client
            .query(query2, &[])
            .await?
            .try_next()
            .await?
            .and_then(|row| row.into_row())
            .and_then(|row| row.get::<i64, _>(0))
            .unwrap_or(0);

        Ok(ClientPage {
            current_page: page,
            total_pages,
            clients,
        })
    }
    async fn get_products(
        &mut self,
        search: String,
        page: i64,
    ) -> Result<ProductPage, Box<dyn std::error::Error>> {
        let table_inve = env::var("TABLE_INVE").expect("TABLE_INVE is not set");

        let query1 = format!(
            "SELECT CVE_ART as id, DESCR as description, ULT_COSTO as price
                FROM dbo.{}
                WHERE DESCR LIKE '%{}%'
                ORDER BY CVE_ART
                OFFSET {} ROWS
                FETCH NEXT 10 ROWS ONLY;",
            table_inve,
            search,
            (page - 1) * 10,
        );

        let query2 = format!(
            "SELECT (COUNT(*)-1)/10+1 as total_pages
                FROM dbo.{}
                WHERE DESCR LIKE '%{}%';",
            table_inve, search,
        );

        let client = self.client.as_mut().unwrap();

        let products = {
            let mut result = client.query(query1, &[]).await?;
            let mut products = Vec::new();
            while let Some(item) = result.try_next().await? {
                if let Some(row) = item.into_row() {
                    let id: String = row
                        .get::<&str, _>(0)
                        .unwrap_or("unknown_product")
                        .to_string();
                    let description: String = row
                        .get::<&str, _>(1)
                        .unwrap_or("unknown_product")
                        .to_string();
                    let price: f64 = row.get::<f64, _>(2).unwrap_or(0.0);
                    products.push(ProductRow {
                        id,
                        description,
                        price,
                    });
                }
            }
            products
        };

        let total_pages = client
            .query(query2, &[])
            .await?
            .try_next()
            .await?
            .and_then(|row| row.into_row())
            .and_then(|row| row.get::<i64, _>(0))
            .unwrap_or(0);

        Ok(ProductPage {
            current_page: page,
            total_pages,
            products,
        })
    }

    async fn get_client_by_id(
        &mut self,
        id: String,
    ) -> Result<ClientRow, Box<dyn std::error::Error>> {
        let table_client = env::var("TABLE_CLIENT").expect("TABLE_CLIENT is not set");
        let query = format!(
            "SELECT CLAVE as id, NOMBRE as name, EMAILPRED as email
             FROM dbo.{}
             WHERE CLAVE = '{}';",
            table_client, id
        );
        let client = self.client.as_mut().unwrap();
        let client_row = client
            .query(query, &[])
            .await?
            .try_next()
            .await?
            .and_then(|row| row.into_row())
            .map(|row| {
                let id: String = row.get::<&str, _>(0).unwrap_or("unknown_id").to_string();
                let name: String = row.get::<&str, _>(1).unwrap_or("unknown_name").to_string();
                let email: String = row.get::<&str, _>(2).unwrap_or("unknown_email").to_string();
                ClientRow { id, name, email }
            })
            .ok_or_else(|| "Client not found".into());
        client_row
    }

    async fn get_product_by_id(
        &mut self,
        id: String,
    ) -> Result<ProductRow, Box<dyn std::error::Error>> {
        let table_inve = env::var("TABLE_INVE").expect("TABLE_INVE is not set");
        let query = format!(
            "SELECT CVE_ART as id, DESCR as description, ULT_COSTO as price
             FROM dbo.{}
             WHERE CVE_ART = '{}';",
            table_inve, id
        );
        let client = self.client.as_mut().unwrap();
        let client_row = client
            .query(query, &[])
            .await?
            .try_next()
            .await?
            .and_then(|row| row.into_row())
            .map(|row| {
                let id: String = row.get::<&str, _>(0).unwrap_or("unknown_id").to_string();
                let description: String =
                    row.get::<&str, _>(1).unwrap_or("unknown_name").to_string();
                let price: f64 = row.get::<f64, _>(2).unwrap_or(0.0);
                ProductRow {
                    id,
                    description,
                    price,
                }
            })
            .ok_or_else(|| "Client not found".into());
        client_row
    }
}
