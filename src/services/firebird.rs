use crate::database::DatabaseTrait;
use async_trait::async_trait;
use rsfbclient::{Connection, ConnectionBuilder, FirebirdClient, ParamsBuilder, Row};
use std::collections::HashMap;

pub struct FirebirdDatabase {
    conn: Connection,
}

impl FirebirdDatabase {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let conn = ConnectionBuilder::builder()
            .host("localhost")
            .db_name("test.fdb")
            .user("SYSDBA")
            .password("masterkey")
            .connect()?;

        Ok(FirebirdDatabase { conn })
    }
}

#[async_trait]
impl DatabaseTrait for FirebirdDatabase {
    async fn build_client_product_matrix(
        &mut self,
    ) -> Result<ClientProductMatrix, Box<dyn std::error::Error>> {
        let sql = "SELECT CLIENT_ID, PRODUCT_ID, SUM(QUANTITY) FROM SALES WHERE STATUS <> 'C' GROUP BY CLIENT_ID, PRODUCT_ID";
        let mut matrix: ClientProductMatrix = HashMap::new();

        let mut rows = self.conn.query(sql, ())?;
        while let Some(row) = rows.next()? {
            let client_id: String = row.get(0).unwrap_or("unknown_client".to_string());
            let product_id: String = row.get(1).unwrap_or("unknown_product".to_string());
            let total_quantity: i32 = row.get(2).unwrap_or(0);

            matrix
                .entry(client_id)
                .or_insert_with(HashMap::new)
                .insert(product_id, total_quantity);
        }

        Ok(matrix)
    }
}
