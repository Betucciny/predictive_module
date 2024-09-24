use crate::services::cache::{get_recommendations, RecommendationCache};
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::Filter;

pub async fn recommendation_handler(
    cache: RecommendationCache,
) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("recommend" / String)
        .and(with_cache(cache.clone()))
        .and_then(get_recommendation)
}

fn with_cache(
    cache: RecommendationCache,
) -> impl Filter<Extract = (RecommendationCache,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || cache.clone())
}

async fn get_recommendation(
    client_id: String,
    cache: RecommendationCache,
) -> Result<impl warp::Reply, warp::Rejection> {
    if let Some(recommendations) = get_recommendations(cache, &client_id).await {
        Ok(warp::reply::json(&recommendations))
    } else {
        Ok(warp::reply::json(&Vec::<String>::new()))
    }
}
