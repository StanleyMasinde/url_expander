use std::{
    fmt::Display,
    path::PathBuf,
    sync::{Arc, OnceLock},
    time::{Instant, SystemTime},
};

use dashmap::DashMap;
use thiserror::Error;

pub static DISK_CACHE: OnceLock<Cache> = OnceLock::new();
pub static CACHE_DIR: &str = "url_expander";

#[derive(Clone)]
pub struct RateLimiter {
    pub buckets: Arc<DashMap<String, Bucket>>,
}

#[derive(Clone)]
pub struct Bucket {
    pub(crate) tokens: f64,
    pub(crate) last_refill: Instant,
}

#[derive(Error, Debug)]
pub(crate) enum CacheError {
    #[error("Item not found in cache.")]
    NotFound,

    #[error("Path not found.")]
    FileNotFound { path: PathBuf },

    #[error("Cache directory not available.")]
    CacheDirUnavailable,

    #[error("An unknown error occurred.")]
    UknownError,
}

pub type CacheResult<T> = Result<T, CacheError>;

#[derive(Clone)]
pub(crate) enum Storage {
    Memory,
    Disk,
}

pub trait Transport {
    fn prune(&self) -> CacheResult<bool>;
    fn set<V>(&self, key: &str, value: V) -> CacheResult<bool>
    where
        V: Cacheable;
    fn get(&self, key: &str) -> CacheResult<Option<String>>;
}

#[derive(Debug)]
pub struct CacheItem {
    pub(crate) value: String,
    pub(crate) last_update: SystemTime,
}

impl From<&str> for CacheItem {
    fn from(value: &str) -> Self {
        Self {
            value: value.to_string(),
            last_update: SystemTime::now(),
        }
    }
}

impl Display for CacheItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl From<String> for CacheItem {
    fn from(value: String) -> Self {
        Self {
            value,
            last_update: SystemTime::now(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct Cache {
    pub entries: Arc<DashMap<String, CacheItem>>,
    pub storage: Storage,
}

pub trait Cacheable {
    fn to_cache_value(self) -> CacheItem;
}

impl Cacheable for String {
    fn to_cache_value(self) -> CacheItem {
        CacheItem {
            value: self,
            last_update: SystemTime::now(),
        }
    }
}

impl Cacheable for &str {
    fn to_cache_value(self) -> CacheItem {
        CacheItem {
            value: self.to_string(),
            last_update: SystemTime::now(),
        }
    }
}
