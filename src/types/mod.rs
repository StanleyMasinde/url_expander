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
    /// Creates a CacheItem with the provided string as its value and records the current system time as `last_update`.
    ///
    /// # Examples
    ///
    /// ```
    /// let item = CacheItem::from("hello");
    /// assert_eq!(item.value, "hello");
    /// ```
    fn from(value: &str) -> Self {
        Self {
            value: value.to_string(),
            last_update: SystemTime::now(),
        }
    }
}

impl Display for CacheItem {
    /// Formats the cache item's stored string value for display.
    ///
    /// # Examples
    ///
    /// ```
    /// let item = crate::types::CacheItem::from("cached-value");
    /// assert_eq!(format!("{}", item), "cached-value");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl From<String> for CacheItem {
    /// Creates a CacheItem from a `String`, setting `last_update` to the current system time.
    ///
    /// # Examples
    ///
    /// ```
    /// let s = "hello".to_string();
    /// let item = CacheItem::from(s.clone());
    /// assert_eq!(item.value, s);
    /// ```
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
    /// Converts the `String` into a `CacheItem` and records the current system time as `last_update`.
    ///
    /// # Examples
    ///
    /// ```
    /// let item = String::from("hello").to_cache_value();
    /// assert_eq!(item.value, "hello");
    /// ```
    fn to_cache_value(self) -> CacheItem {
        CacheItem {
            value: self,
            last_update: SystemTime::now(),
        }
    }
}

impl Cacheable for &str {
    /// Converts the string slice into a CacheItem holding the string and the current timestamp.
    ///
    /// Produces a CacheItem whose `value` is the string content and whose `last_update` is set to `SystemTime::now()`.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::types::Cacheable;
    ///
    /// let item = "hello".to_cache_value();
    /// assert_eq!(item.value, "hello");
    /// ```
    fn to_cache_value(self) -> CacheItem {
        CacheItem {
            value: self.to_string(),
            last_update: SystemTime::now(),
        }
    }
}