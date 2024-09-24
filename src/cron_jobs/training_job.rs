use crate::services::{
    als::ALS,
    cache::{update_cache, RecommendationCache},
    db::Database,
};
use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler};

pub async fn schedule_training_job(
    cache: RecommendationCache,
    als: ALS,
    db: Database,
) -> Result<(), Box<dyn std::error::Error>> {
    let sched = JobScheduler::new().await?;

    let job = Job::new_async("0 0 * * * *", move |_uuid, _l| {
        let cache = Arc::clone(&cache);
        let db = db.clone();
        let als = als.clone();

        Box::pin(async move {
            println!("Running ALS training job");
            let matrix = db.build_client_product_matrix().await.unwrap();
            let (client_factors, product_factors) = als.fit(&matrix);

            let mut recommendations: HashMap<String, Vec<String>> = HashMap::new();
            for client_id in matrix.keys() {
                let recs = als.recommend(client_id, &client_factors, &product_factors);
                recommendations.insert(client_id.clone(), recs);
            }

            update_cache(cache, recommendations).await;
        })
    })?;

    sched.add(job).await?;
    sched.start().await?;

    Ok(())
}
