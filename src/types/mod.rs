use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Instant,
};

#[derive(Clone)]
pub struct RateLimiter {
    pub buckets: Arc<Mutex<HashMap<String, Bucket>>>,
}

#[derive(Clone)]
pub struct Bucket {
    pub(crate) tokens: f64,
    pub(crate) last_refill: Instant,
}
