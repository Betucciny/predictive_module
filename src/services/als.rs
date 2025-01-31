use crate::models::db::ClientProductMatrix;
use ndarray::{s, Array1, Array2, Axis};
use ndarray_linalg::Solve;
use ndarray_rand::{rand_distr::Uniform, RandomExt};
use std::collections::HashMap;
use std::collections::HashSet;

pub struct ALS {
    pub num_factors: usize,
    pub regularization: f64,
    pub confidence_multiplier: f64,
    pub tolerance: f64,
    pub max_iterations: usize,
    pub matrix: ClientProductMatrix,

    pub client_factors: Option<Array2<f64>>,
    pub product_factors: Option<Array2<f64>>,
    client_index: Option<HashMap<String, usize>>,
    product_index: Option<HashMap<String, usize>>,
}

impl ALS {
    pub fn new(
        num_factors: usize,
        regularization: f64,
        confidence_multiplier: f64,
        tolerance: f64,
        max_iterations: usize,
        matrix: ClientProductMatrix,
    ) -> Self {
        ALS {
            num_factors,
            regularization,
            confidence_multiplier,
            tolerance,
            max_iterations,
            matrix: matrix.clone(),
            client_factors: None,
            product_factors: None,
            client_index: None,
            product_index: None,
        }
    }

    fn build_rating_matrix(&mut self) -> Array2<f64> {
        let clients: HashSet<_> = self.matrix.keys().collect();
        let products: HashSet<_> = self.matrix.values().flat_map(|p| p.keys()).collect();

        let client_index: HashMap<_, _> = clients
            .iter()
            .enumerate()
            .map(|(i, &c)| (c.clone(), i))
            .collect();
        let product_index: HashMap<_, _> = products
            .iter()
            .enumerate()
            .map(|(i, &p)| (p.clone(), i))
            .collect();

        let mut rating_matrix = Array2::<f64>::zeros((products.len(), clients.len()));
        for (client, products) in &self.matrix {
            let client_idx = client_index[client];
            for (product, &quantity) in products {
                let product_idx = product_index[product];
                rating_matrix[(product_idx, client_idx)] = quantity;
            }
        }
        self.client_index = Some(client_index);
        self.product_index = Some(product_index);

        rating_matrix
    }

    fn create_weight_matrix(&self, matrix: &Array2<f64>) -> Array2<f64> {
        let binary_matrix = matrix.mapv(|r| if r > 0.0 { r } else { 0.0 });
        let weighted_matrix = binary_matrix.mapv(|r| 1.0 + self.confidence_multiplier * r);
        weighted_matrix
    }

    pub fn fit(&mut self) {
        let rating_matrix = self.build_rating_matrix();
        let weighted_matrix = self.create_weight_matrix(&rating_matrix);

        let num_clients = weighted_matrix.shape()[1];
        let num_products = weighted_matrix.shape()[0];

        let mut client_factors =
            Array2::<f64>::random((num_clients, self.num_factors), Uniform::new(0.0, 1.0));
        let mut product_factors =
            Array2::<f64>::random((num_products, self.num_factors), Uniform::new(0.0, 1.0));

        for _ in 0..self.max_iterations {
            // Fix product_factors and solve for client_factors
            for i in 0..num_clients {
                let ratings = weighted_matrix.slice(s![.., i]);
                let non_zero_indices: Vec<usize> = ratings
                    .indexed_iter()
                    .filter(|(_, &v)| v > 0.0)
                    .map(|(idx, _)| idx)
                    .collect();
                if non_zero_indices.is_empty() {
                    continue;
                }
                let sub_matrix = product_factors.select(Axis(0), &non_zero_indices);
                let sub_ratings = ratings.select(Axis(0), &non_zero_indices);
                let regularization_matrix =
                    Array2::<f64>::eye(self.num_factors) * self.regularization;
                let lhs = sub_matrix.t().dot(&sub_matrix) + &regularization_matrix;
                let rhs = sub_matrix.t().dot(&sub_ratings);
                client_factors
                    .slice_mut(s![i, ..])
                    .assign(&lhs.solve_h_into(rhs).unwrap());
            }

            // Fix client_factors and solve for product_factors
            for j in 0..num_products {
                let ratings = weighted_matrix.slice(s![j, ..]);
                let non_zero_indices: Vec<usize> = ratings
                    .indexed_iter()
                    .filter(|(_, &v)| v > 0.0)
                    .map(|(idx, _)| idx)
                    .collect();
                if non_zero_indices.is_empty() {
                    continue;
                }
                let sub_matrix = client_factors.select(Axis(0), &non_zero_indices);
                let sub_ratings = ratings.select(Axis(0), &non_zero_indices);
                let regularization_matrix =
                    Array2::<f64>::eye(self.num_factors) * self.regularization;
                let lhs = sub_matrix.t().dot(&sub_matrix) + &regularization_matrix;
                let rhs = sub_matrix.t().dot(&sub_ratings);
                product_factors
                    .slice_mut(s![j, ..])
                    .assign(&lhs.solve_into(rhs).unwrap());
            }
        }

        self.client_factors = Some(client_factors);
        self.product_factors = Some(product_factors);
    }

    pub fn recommend(&self, client_id: &str, n: Option<usize>) -> Vec<String> {
        if let (
            Some(ref client_factors),
            Some(ref product_factors),
            Some(ref client_index),
            Some(ref product_index),
        ) = (
            &self.client_factors,
            &self.product_factors,
            &self.client_index,
            &self.product_index,
        ) {
            if let Some(&client_idx) = client_index.get(client_id) {
                let client_vector = client_factors.row(client_idx);
                let mut product_scores: Vec<(String, f64)> = product_index
                    .iter()
                    .map(|(product_id, &product_idx)| {
                        let product_vector = product_factors.row(product_idx);
                        let score = client_vector.dot(&product_vector);
                        (product_id.clone(), score)
                    })
                    .collect();

                product_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

                let top_n = n.unwrap_or(1);
                return product_scores
                    .into_iter()
                    .take(top_n)
                    .map(|(product_id, _)| product_id)
                    .collect();
            }
            log::warn!("Client ID not found: {}", client_id);
            return Vec::new();
        }
        log::warn!("Model not trained yet");
        Vec::new()
    }

    pub fn compute_epr(&self) -> Option<f64> {
        if let (
            Some(ref client_factors),
            Some(ref product_factors),
            Some(ref client_index),
            Some(ref product_index),
        ) = (
            &self.client_factors,
            &self.product_factors,
            &self.client_index,
            &self.product_index,
        ) {
            let mut total_percentile_rank = 0.0;
            let mut count = 0;

            for (client, products) in &self.matrix {
                if let Some(&client_idx) = client_index.get(client) {
                    let client_vector = client_factors.row(client_idx);
                    let mut product_scores: Vec<(usize, f64)> = product_index
                        .iter()
                        .map(|(_, &product_idx)| {
                            let product_vector = product_factors.row(product_idx);
                            let score = client_vector.dot(&product_vector);
                            (product_idx, score)
                        })
                        .collect();

                    product_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

                    for (product, _) in products {
                        if let Some(&product_idx) = product_index.get(product) {
                            let rank = product_scores
                                .iter()
                                .position(|&(idx, _)| idx == product_idx)
                                .unwrap();
                            let percentile_rank = rank as f64 / product_scores.len() as f64;
                            total_percentile_rank += percentile_rank;
                            count += 1;
                        }
                    }
                }
            }

            if count > 0 {
                Some(total_percentile_rank / count as f64)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn build_from_data(
        &mut self,
        client_factors: &Vec<Vec<f64>>,
        product_factors: &Vec<Vec<f64>>,
    ) {
        self.build_rating_matrix();

        let num_clients = client_factors.len();
        let num_products = product_factors.len();

        let mut client_factors_array = Array2::<f64>::zeros((num_clients, self.num_factors));
        for (i, factors) in client_factors.iter().enumerate() {
            client_factors_array
                .row_mut(i)
                .assign(&Array1::from_vec(factors.clone()));
        }

        let mut product_factors_array = Array2::<f64>::zeros((num_products, self.num_factors));
        for (i, factors) in product_factors.iter().enumerate() {
            product_factors_array
                .row_mut(i)
                .assign(&Array1::from_vec(factors.clone()));
        }

        self.client_factors = Some(client_factors_array);
        self.product_factors = Some(product_factors_array);
    }
}
