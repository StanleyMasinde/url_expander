use std::time::Duration;

use log::{debug, warn};
use tokio::time::interval;

use crate::types::{Cache, Transport};

pub async fn job_runner(app_cache: Cache) {
    let mut ticker = interval(Duration::from_secs(60));

    loop {
        ticker.tick().await;
        if let Err(e) = app_cache.prune() {
            warn!("Failed to prune cache: {}", e)
        };
        debug!("Job run")
    }
}
