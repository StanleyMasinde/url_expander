use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

use dashmap::DashMap;

#[derive(Clone)]
pub struct RateLimiter {
    pub buckets: Arc<Mutex<DashMap<String, Bucket>>>,
}

#[derive(Clone)]
pub struct Bucket {
    pub(crate) tokens: f64,
    pub(crate) last_refill: Instant,
}
