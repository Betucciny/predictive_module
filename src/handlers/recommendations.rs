use crate::MODEL_SERVER;
use percent_encoding::percent_decode_str;
use warp::Filter;

pub fn global_handler(
) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    get_recommendation_with_limit()
        .or(get_recommendation())
        .or(metadata_handler())
        .or(clients_handler())
        .or(products_handler())
}

fn metadata_handler(
) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path("metadata").and_then(|| async move {
        println!("Received request for metadata");
        let model_server = MODEL_SERVER.lock().await;
        let metadata = model_server.as_ref().unwrap().get_metadata().await;
        Result::<_, warp::Rejection>::Ok(warp::reply::json(&metadata))
    })
}

fn clients_handler(
) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path("clients")
        .and(warp::query::<std::collections::HashMap<String, String>>())
        .and_then(
            |query: std::collections::HashMap<String, String>| async move {
                let search = query.get("search").cloned().unwrap_or_default();
                let page = query
                    .get("page")
                    .and_then(|p| p.parse::<i64>().ok())
                    .unwrap_or(1);
                println!(
                    "Received request for clients with search: {} and page: {}",
                    search, page
                );
                let model_server = MODEL_SERVER.lock().await;
                match model_server
                    .as_ref()
                    .unwrap()
                    .get_clients(search, page)
                    .await
                {
                    Ok(client_page) => Ok(warp::reply::json(&client_page)),
                    Err(_) => Err(warp::reject::not_found()),
                }
            },
        )
}

fn products_handler(
) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path("products")
        .and(warp::query::<std::collections::HashMap<String, String>>())
        .and_then(
            |query: std::collections::HashMap<String, String>| async move {
                let search = query.get("search").cloned().unwrap_or_default();
                let page = query
                    .get("page")
                    .and_then(|p| p.parse::<i64>().ok())
                    .unwrap_or(1);
                println!(
                    "Received request for products with search: {} and page: {}",
                    search, page
                );
                let model_server = MODEL_SERVER.lock().await;
                match model_server
                    .as_ref()
                    .unwrap()
                    .get_products(search, page)
                    .await
                {
                    Ok(product_page) => Ok(warp::reply::json(&product_page)),
                    Err(e) => {
                        eprintln!("Error getting products: {:?}", e);
                        Err(warp::reject::not_found())
                    }
                }
            },
        )
}

fn get_recommendation(
) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path("recommend")
        .and(warp::path::param::<String>())
        .and_then(|client_id: String| async move {
            println!(
                "Received request for recommendations for client_id: {}",
                client_id
            );
            let decoded_client_id = percent_decode_str(&client_id)
                .decode_utf8_lossy()
                .to_string();
            let model_server = MODEL_SERVER.lock().await;
            match model_server
                .as_ref()
                .unwrap()
                .predict(decoded_client_id.as_str(), None)
                .await
            {
                Some(recommendations) => Ok(warp::reply::json(&recommendations)),
                None => Err(warp::reject::not_found()),
            }
        })
}

fn get_recommendation_with_limit(
) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path("recommend")
        .and(warp::path::param::<String>())
        // .and(warp::path("limit"))
        .and(warp::path::param::<i64>())
        .and_then(|client_id: String, limit: i64| async move {
            println!(
                "Received request for recommendations for client_id: {} with limit: {}",
                client_id, limit
            );
            let decoded_client_id = percent_decode_str(&client_id)
                .decode_utf8_lossy()
                .to_string();
            let model_server = MODEL_SERVER.lock().await;
            match model_server
                .as_ref()
                .unwrap()
                .predict(decoded_client_id.as_str(), Some(limit as usize))
                .await
            {
                Some(recommendations) => Ok(warp::reply::json(&recommendations)),
                None => Err(warp::reject::not_found()),
            }
        })
}
