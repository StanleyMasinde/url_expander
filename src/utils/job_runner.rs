use std::time::Duration;

use log::debug;
use tokio::time::interval;

use crate::utils::cache::{Cache, Transport};

pub async fn job_runner() {
    let app_cache = Cache::new();
    let mut ticker = interval(Duration::from_secs(60));

    loop {
        ticker.tick().await;
        // I am not sure how in memory will work. It should not be a problem since the base cache
        // checks this.
        app_cache.prune().unwrap();
        debug!("Job run")
    }
}
