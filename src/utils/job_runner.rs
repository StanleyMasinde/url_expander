use std::time::Duration;

use log::debug;
use tokio::time::interval;

use crate::types::{Cache, Transport};

pub async fn job_runner(app_cache: Cache) {
    let mut ticker = interval(Duration::from_secs(60));

    loop {
        ticker.tick().await;
        app_cache.prune().unwrap();
        debug!("Job run")
    }
}
