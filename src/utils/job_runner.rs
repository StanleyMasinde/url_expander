use std::time::Duration;

use tokio::time::interval;

use crate::utils::cache::{Cache, Transport};

pub async fn job_runner() {
    let app_cache = Cache::new();
    let mut ticker = interval(Duration::from_secs(60));

    loop {
        ticker.tick().await;
        app_cache.prune().unwrap();
        println!("Job run")
    }
}
