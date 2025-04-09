use crate::models::db::ClientProductMatrix;
use crate::services::als::ALS;
use futures::FutureExt;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::fs::File;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Notify;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hyperparameters {
    pub num_factors: usize,
    pub regularization: f64,
    pub confidence_multiplier: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JSONData {
    pub hyperparameters: Hyperparameters,
    pub matrix: ClientProductMatrix,
    pub product_factors: Vec<Vec<f64>>,
    pub client_factors: Vec<Vec<f64>>,
    pub client_index: HashMap<String, usize>,
    pub product_index: HashMap<String, usize>,
}

fn generate_hyperparameter_combinations(
    num_factors: &[usize],
    regularization: &[f64],
    confidence_multiplier: &[f64],
) -> Vec<Hyperparameters> {
    num_factors
        .iter()
        .flat_map(|&num_factors| {
            regularization.iter().flat_map(move |&regularization| {
                confidence_multiplier
                    .iter()
                    .map(move |&confidence_multiplier| Hyperparameters {
                        num_factors,
                        regularization,
                        confidence_multiplier,
                    })
            })
        })
        .collect()
}

pub async fn find_best_als_model(
    matrix: ClientProductMatrix,
    notify: Arc<Notify>,
) -> Option<Hyperparameters> {
    println!("Finding best ALS model...");
    let num_factors = vec![20, 50, 100, 200];
    let regularization = vec![0.01, 0.1];
    let confidence_multiplier = vec![20.0, 40.0, 60.0];
    // let num_factors = vec![20];
    // let regularization = vec![0.01];
    // let confidence_multiplier = vec![20.0];

    let hyperparameter_combinations =
        generate_hyperparameter_combinations(&num_factors, &regularization, &confidence_multiplier);

    let total_combinations = hyperparameter_combinations.len();

    println!("Total combinations: {}", total_combinations);

    let processed_counter = Arc::new(AtomicUsize::new(0));

    let start_time = Instant::now();

    let (
        best_hyperparameters,
        best_epr,
        best_client_factors,
        best_product_factors,
        best_client_index,
        best_product_index,
    ) = hyperparameter_combinations
        .par_iter()
        .filter_map(|hyperparameters| {
            let notify = notify.clone();
            if notify.notified().now_or_never().is_some() {
                println!("Cancellation requested, stopping find_best_als_model");
                return None;
            }

            let matrix_clone = matrix.clone(); // Clone the matrix for each ALS instance
            let mut als = ALS::new(
                hyperparameters.num_factors,
                hyperparameters.regularization,
                hyperparameters.confidence_multiplier,
                1e-4,
                200,
                matrix_clone,
            );
            als.fit(notify.clone());
            let epr = als.compute_epr().unwrap();

            let processed = processed_counter.fetch_add(1, Ordering::SeqCst) + 1;
            println!(
                "Processed {}/{} combinations EPR: {:.2}% ({:.2}%) | Metadata: num_factors: {}, regularization: {}, confidence_multiplier: {}",
                processed,
                total_combinations,
                epr * 100.0,
                (processed as f64 / total_combinations as f64) * 100.0,
                hyperparameters.num_factors,
                hyperparameters.regularization,
                hyperparameters.confidence_multiplier
            );

            let product_factors: Vec<Vec<f64>> = als
                .product_factors
                .clone()
                .unwrap()
                .outer_iter()
                .map(|row| row.to_vec())
                .collect();
            let client_factors: Vec<Vec<f64>> = als
                .client_factors
                .clone()
                .unwrap()
                .outer_iter()
                .map(|row| row.to_vec())
                .collect();

            let client_index = als.client_index.clone().unwrap();
            let product_index = als.product_index.clone().unwrap();

            Some((
                hyperparameters.clone(),
                epr,
                client_factors,
                product_factors,
                client_index,
                product_index,
            ))
        })
        .min_by(|(_, epr1, _, _, _, _), (_, epr2, _, _, _, _)| epr1.partial_cmp(epr2).unwrap())?;

    let elapsed_time = start_time.elapsed();
    println!("Best EPR: {:?}%", best_epr * 100.0);
    println!("Best hyperparameters: {:?}", best_hyperparameters);
    println!(
        "Time taken to process all combinations: {:.2?}",
        elapsed_time
    );

    let json_data = JSONData {
        hyperparameters: Hyperparameters {
            num_factors: best_hyperparameters.num_factors,
            regularization: best_hyperparameters.regularization,
            confidence_multiplier: best_hyperparameters.confidence_multiplier,
        },
        matrix,
        product_factors: best_product_factors,
        client_factors: best_client_factors,
        client_index: best_client_index,
        product_index: best_product_index,
    };

    save_hyperparameters_to_file(&json_data, "./data/hyperparameters.json")
        .expect("Failed to save hyperparameters");

    Some(best_hyperparameters)
}

fn save_hyperparameters_to_file(
    data: &JSONData,
    file_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create(file_path)?;
    serde_json::to_writer(file, data)?;
    Ok(())
}
