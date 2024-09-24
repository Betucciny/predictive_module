use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type RecommendationCache = Arc<Mutex<HashMap<String, Vec<String>>>>;

// Update the recommendation cache after training
pub async fn update_cache(
    cache: RecommendationCache,
    recommendations: HashMap<String, Vec<String>>,
) {
    let mut cache_lock = cache.lock().await;
    *cache_lock = recommendations;
}

// Retrieve recommendations for a client from the cache
pub async fn get_recommendations(
    cache: RecommendationCache,
    client_id: &String,
) -> Option<Vec<String>> {
    let cache_lock = cache.lock().await;
    cache_lock.get(client_id).cloned()
}
