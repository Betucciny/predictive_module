use crate::models::db::{ClientPage, Database, DatabaseError, DatabaseTrait, ProductPage};
use crate::services::als::ALS;
use crate::services::firebird::FirebirdDatabase;
use crate::services::mssql::SqlServerDatabase;
use crate::services::training::find_best_als_model;
use notify::{recommended_watcher, EventKind, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, Mutex as TokioMutex, Notify};
use tokio::time::{sleep, Duration};

use super::training::JSONData;

pub struct ModelServer {
    model: Arc<Mutex<Option<ALS>>>,
    hyperparameters_file: String,
    notify: Option<Arc<Notify>>,
    db: Option<Arc<TokioMutex<Database>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataModel {
    num_factors: usize,
    regularization: f64,
    confidence_multiplier: f64,
    epr: f64,
}

impl ModelServer {
    pub fn new(hyperparameters_file: &str) -> Arc<TokioMutex<Self>> {
        Arc::new(TokioMutex::new(ModelServer {
            model: Arc::new(Mutex::new(None)),
            hyperparameters_file: hyperparameters_file.to_string(),
            notify: None,
            db: None,
        }))
    }

    pub async fn initialize(
        &mut self,
        notify: Arc<Notify>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.notify = Some(notify.clone());
        let db_type = std::env::var("DB_TYPE").expect("DB_TYPE is not set in the environment");
        let db: Arc<TokioMutex<dyn DatabaseTrait + Send + Sync>> = match db_type.as_str() {
            "sqlserver" => Arc::new(TokioMutex::new(SqlServerDatabase::new().await)),
            "firebird" => Arc::new(TokioMutex::new(FirebirdDatabase::new())),
            _ => panic!("Unsupported DB_TYPE: '{}'", db_type),
        };
        self.db = Some(Arc::new(TokioMutex::new(Database::new(db))));
        if let Ok(json_data) = load_json_data_from_file(&self.hyperparameters_file) {
            let mut model = ALS::new(
                json_data.hyperparameters.num_factors,
                json_data.hyperparameters.regularization,
                json_data.hyperparameters.confidence_multiplier,
                1e-4,
                100,
                json_data.matrix.clone(),
            );
            model.build_from_data(
                &json_data.client_factors,
                &json_data.product_factors,
                &json_data.client_index,
                &json_data.product_index,
            );
            let mut model_lock = self.model.lock().unwrap();
            *model_lock = Some(model);
        } else {
            println!("Hyperparameters file not found, waiting for file creation...");
            let matrix = self
                .db
                .as_ref()
                .unwrap()
                .lock()
                .await
                .build_matrix()
                .await
                .unwrap();
            let _ = {
                let notify = self.notify.clone();
                tokio::spawn(async move { find_best_als_model(matrix, notify.unwrap()).await })
            };
        }

        self.start_file_watcher();
        return Ok(());
    }

    pub fn predict(&self, user_id: &str, n: Option<usize>) -> Option<Vec<String>> {
        let model = self.model.lock().unwrap();
        if let Some(ref m) = *model {
            let recommendation = m.recommend(user_id, n);
            return Some(recommendation);
        } else {
            return None;
        }
    }

    pub fn get_metadata(&self) -> MetadataModel {
        let model = self.model.lock().unwrap();
        if let Some(ref m) = *model {
            MetadataModel {
                num_factors: m.num_factors,
                regularization: m.regularization,
                confidence_multiplier: m.confidence_multiplier,
                epr: m.compute_epr().unwrap_or(0.0),
            }
        } else {
            MetadataModel {
                num_factors: 0,
                regularization: 0.0,
                confidence_multiplier: 0.0,
                epr: 0.0,
            }
        }
    }

    pub async fn get_clients(
        &self,
        search: String,
        page: i64,
    ) -> Result<ClientPage, DatabaseError> {
        if let Some(ref db) = self.db {
            let mut db = db.lock().await;
            db.get_clients(search, page).await
        } else {
            Err(DatabaseError::ConnectionError(
                "Database not initialized".to_string(),
            ))
        }
    }

    pub async fn get_products(
        &self,
        search: String,
        page: i64,
    ) -> Result<ProductPage, DatabaseError> {
        if let Some(ref db) = self.db {
            let mut db = db.lock().await;
            let products = db.get_products(search, page).await;
            products
        } else {
            Err(DatabaseError::ConnectionError(
                "Database not initialized".to_string(),
            ))
        }
    }

    fn start_file_watcher(&self) {
        let hyperparameters_path = self.hyperparameters_file.clone();
        let hyperparameters_dir = Path::new(&hyperparameters_path)
            .parent()
            .unwrap_or(Path::new("."))
            .to_path_buf();
        let hyperparameters_file = Path::new(&hyperparameters_path)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        let model = self.model.clone();
        let notify = self.notify.clone();

        tokio::spawn(async move {
            let (tx, mut rx) = mpsc::channel(1);
            let mut watcher = match recommended_watcher(move |res| {
                let _ = tx.blocking_send(res);
            }) {
                Ok(watcher) => watcher,
                Err(e) => {
                    eprintln!("Failed to create watcher: {:?}", e);
                    return;
                }
            };

            if let Err(e) = watcher.watch(&hyperparameters_dir, RecursiveMode::NonRecursive) {
                eprintln!("Failed to watch directory: {:?}", e);
                return;
            }

            println!("Started watching directory: {:?}", hyperparameters_dir);
            let notify = notify.as_ref().unwrap().clone();
            loop {
                tokio::select! {
                    _ = notify.notified() => {
                        println!("File watcher received shutdown signal.");
                        break;
                    }
                    res = rx.recv() => {
                        match res {
                            Some(Ok(event)) => {

                                if event
                                    .paths
                                    .iter()
                                    .any(|path| path.ends_with(&hyperparameters_file))
                                    && (matches!(event.kind, EventKind::Modify(_)) || matches!(event.kind, EventKind::Create(_)))
                                {
                                    println!("Hyperparameters file changed or created, reloading model...");
                                    // Add a delay to allow the file writing process to complete
                                    sleep(Duration::from_millis(500)).await;
                                    match load_json_data_from_file(&hyperparameters_path) {
                                        Ok(json_data) => {
                                            let mut model = model.lock().unwrap();
                                            *model = Some(ALS::new(
                                                json_data.hyperparameters.num_factors,
                                                json_data.hyperparameters.regularization,
                                                json_data.hyperparameters.confidence_multiplier,
                                                1e-4,
                                                200,
                                                json_data.matrix.clone(),
                                            ));
                                            if let Some(ref mut m) = *model {
                                                m.build_from_data(
                                                    &json_data.client_factors,
                                                    &json_data.product_factors,
                                                    &json_data.client_index,
                                                    &json_data.product_index,
                                                );
                                            }
                                            println!("Model reloaded successfully.");
                                        }
                                        Err(e) => {
                                            println!("Failed to reload model: {:?}", e);
                                        }
                                    }
                                }
                            }
                            Some(Err(e)) => println!("Watch error: {:?}", e),
                            None => {
                                println!("Channel closed");
                                break;
                            }
                        }
                    }
                }
            }
        });
    }
}

fn load_json_data_from_file(
    file_path: &str,
) -> Result<JSONData, Box<dyn std::error::Error + Send + Sync>> {
    let file = File::open(file_path)?;
    let json_data = serde_json::from_reader(file)?;
    Ok(json_data)
}
