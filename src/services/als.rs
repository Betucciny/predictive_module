use crate::services::db::ClientProductMatrix;
use ndarray::{Array1, Array2};
use std::collections::HashMap;

pub struct ALS {
    pub num_factors: usize,
    pub num_iterations: usize,
    pub regularization: f64,
}

impl ALS {
    pub fn new(num_factors: usize, num_iterations: usize, regularization: f64) -> Self {
        ALS {
            num_factors,
            num_iterations,
            regularization,
        }
    }

    // Train the ALS model
    pub fn fit(&self, matrix: &ClientProductMatrix) -> (Array1<f64>, Array2<f64>) {
        let num_clients = matrix.len();
        let num_products = matrix
            .values()
            .flat_map(|client_map| client_map.keys())
            .collect::<HashMap<_, _>>()
            .len();

        // Initialize random client and product factors
        let mut client_factors = Array1::<f64>::random((num_clients, self.num_factors), 0.0..1.0);
        let mut product_factors = Array2::<f64>::random((num_products, self.num_factors), 0.0..1.0);

        for _ in 0..self.num_iterations {
            // Update factors based on alternating least squares
            // We will add the logic for regularization and solving here
            // ...
        }

        (client_factors, product_factors)
    }

    // Generate recommendations for a given client using fitted factors
    pub fn recommend(
        &self,
        client_id: &String,
        client_factors: &Array2<f64>,
        product_factors: &Array2<f64>,
    ) -> Vec<String> {
        // Recommendation logic using dot product or similar metric
        // For now, just return a placeholder result
        vec!["product_1".to_string(), "product_2".to_string()]
    }
}
