use std::time::Duration;

use log::{debug, warn};
use tokio::time::interval;

use crate::types::{Cache, Transport};

/// Runs a background job that prunes the provided cache every 60 seconds.
///
/// This function never returns: it awaits a 60-second interval tick in a loop, calls `app_cache.prune()`,
/// logs a warning if pruning fails, and logs a debug message after each run. Spawn it on an async runtime
/// if you want it to run concurrently with other tasks.
///
/// # Parameters
///
/// - `app_cache`: the cache instance to prune periodically.
///
/// # Examples
///
/// ```
/// use tokio::task;
///
/// // `cache` must be moved into the spawned task; clone if needed.
/// let cache = /* obtain Cache instance */;
/// task::spawn(async move {
///     job_runner(cache).await;
/// });
/// ```
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