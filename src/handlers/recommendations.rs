use crate::MODEL_SERVER;
use percent_encoding::percent_decode_str;
use warp::Filter;

pub fn global_handler(
) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let recommendation_route = recommendation_handler();
    let metadata_route = metadata_handler();

    recommendation_route.or(metadata_route)
}

fn recommendation_handler(
) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let base_route = warp::path("recommend");

    let route_with_limit = base_route
        .and(warp::path::param::<String>())
        .and(warp::path::param::<usize>())
        .and_then(get_recommendation_with_limit);

    let route_without_limit = base_route
        .and(warp::path::param::<String>())
        .and_then(get_recommendation);

    route_with_limit.or(route_without_limit)
}

fn metadata_handler(
) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path("metadata").and_then(get_metadata)
}

async fn get_recommendation(client_id: String) -> Result<impl warp::Reply, warp::Rejection> {
    println!(
        "Received request for recommendations for client_id: {}",
        client_id
    );
    let decoded_client_id = percent_decode_str(&client_id)
        .decode_utf8_lossy()
        .to_string();
    match MODEL_SERVER.predict(decoded_client_id.as_str(), None) {
        Some(recommendations) => Ok(warp::reply::json(&recommendations)),
        None => Err(warp::reject::not_found()),
    }
}

async fn get_recommendation_with_limit(
    client_id: String,
    limit: usize,
) -> Result<impl warp::Reply, warp::Rejection> {
    println!(
        "Received request for recommendations for client_id: {} with limit: {}",
        client_id, limit
    );
    let decoded_client_id = percent_decode_str(&client_id)
        .decode_utf8_lossy()
        .to_string();
    match MODEL_SERVER.predict(decoded_client_id.as_str(), Some(limit)) {
        Some(recommendations) => Ok(warp::reply::json(&recommendations)),
        None => Err(warp::reject::not_found()),
    }
}

async fn get_metadata() -> Result<impl warp::Reply, warp::Rejection> {
    let metadata = MODEL_SERVER.get_metadata();
    Ok(warp::reply::json(&metadata))
}
