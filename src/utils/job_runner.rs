use std::time::Duration;

use log::debug;
use tokio::time::interval;

use crate::utils::cache::{Cache, Transport};

/// Runs a background loop that prunes the in-memory cache every 60 seconds.
///
/// This function loops indefinitely and will panic if a cache prune operation returns an error.
///
/// # Examples
///
/// ```
/// # use tokio::time::{sleep, Duration};
/// # // Spawn the runner and stop it shortly after to avoid an infinite test.
/// # tokio::spawn(async {
/// #     // Replace with the actual path to `job_runner` if needed:
/// #     crate::utils::job_runner::job_runner().await;
/// # });
/// # // Let the spawned task run briefly, then return from the example.
/// # tokio::task::yield_now().await;
/// # sleep(Duration::from_millis(10)).await;
/// ```
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