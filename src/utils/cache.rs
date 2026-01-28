use std::{
    fmt::Display,
    fs::{create_dir_all, read_to_string, remove_file, write},
    path::PathBuf,
    sync::Arc,
    time::{Duration, SystemTime},
};

static CACHE_DIR: &str = "url_expander";

use dashmap::DashMap;
use log::debug;
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum CacheError {
    #[error("Item not found in cache.")]
    NotFound,

    #[error("Path not found.")]
    FileNotFound { path: PathBuf },

    #[error("Cache directory not available.")]
    CacheDirUnavailable,

    #[error("An unknown error occoured.")]
    UknownError,
}

#[derive(Clone)]
pub(crate) enum Storage {
    Memory,
    Disk,
}

type CacheResult<T> = Result<T, CacheError>;

pub trait Transport {
    fn prune(&self) -> CacheResult<bool>;
    fn set<V>(&self, key: &str, value: V) -> CacheResult<bool>
    where
        V: Cacheable;
    fn get(&self, key: &str) -> CacheResult<Option<String>>;
}

#[derive(Debug)]
pub struct CacheItem {
    value: String,
    last_update: SystemTime,
}

impl From<&str> for CacheItem {
    /// Creates a `CacheItem` from a string slice and sets `last_update` to the current system time.
    ///
    /// # Examples
    ///
    /// ```
    /// let item = CacheItem::from("hello");
    /// assert_eq!(item.value, "hello");
    /// assert!(std::time::SystemTime::now().duration_since(item.last_update).is_ok());
    /// ```
    fn from(value: &str) -> Self {
        Self {
            value: value.to_string(),
            last_update: SystemTime::now(),
        }
    }
}

impl Display for CacheItem {
    /// Formats the `CacheItem` by writing its inner `value` string.
    ///
    /// # Examples
    ///
    /// ```
    /// let item = CacheItem { value: "expanded".to_string(), last_update: std::time::SystemTime::now() };
    /// assert_eq!(format!("{}", item), "expanded");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl From<String> for CacheItem {
    /// Creates a `CacheItem` from the provided `String` and sets `last_update` to the current system time.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::utils::cache::CacheItem;
    /// let item = CacheItem::from("value".to_string());
    /// assert_eq!(item.value, "value");
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
    entries: Arc<DashMap<String, CacheItem>>,
    storage: Storage,
}

pub trait Cacheable {
    fn to_cache_value(self) -> CacheItem;
}

impl Cacheable for String {
    /// Converts the `String` into a `CacheItem` using the current system time as `last_update`.
    ///
    /// # Examples
    ///
    /// ```
    /// let s = String::from("value");
    /// let item = s.to_cache_value();
    /// assert_eq!(item.value, "value");
    /// ```
    fn to_cache_value(self) -> CacheItem {
        CacheItem {
            value: self,
            last_update: SystemTime::now(),
        }
    }
}

impl Cacheable for &str {
    /// Create a CacheItem from the string, using the current system time as `last_update`.
    ///
    /// The returned `CacheItem` holds the original string as `value` and the time of conversion as `last_update`.
    ///
    /// # Examples
    ///
    /// ```
    /// let item = String::from("hello").to_cache_value();
    /// assert_eq!(item.value, "hello");
    /// ```
    fn to_cache_value(self) -> CacheItem {
        CacheItem {
            value: self.to_string(),
            last_update: SystemTime::now(),
        }
    }
}

impl Cache {
    /// Creates a new in-memory cache with no entries.
    ///
    /// # Examples
    ///
    /// ```
    /// let cache = Cache::new();
    /// // store and retrieve a value
    /// cache.set("key", "value").unwrap();
    /// let val = cache.get("key").unwrap();
    /// assert_eq!(val, Some("value".to_string()));
    /// ```
    pub fn new() -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
            storage: Storage::Memory,
        }
    }

    /// Sets the storage backend for this cache and returns the configured instance.
    ///
    /// `store` specifies the storage backend to use for subsequent cache operations.
    ///
    /// # Returns
    ///
    /// The cache instance configured to use `store`.
    ///
    /// # Examples
    ///
    /// ```
    /// let _ = Cache::new().with_storage(Storage::Disk);
    /// ```
    pub fn with_storage(mut self, store: Storage) -> Self {
        self.storage = store;
        self
    }

    /// Compute the SHA-256 digest of `key` and return it as a lowercase hexadecimal string.
    ///
    /// # Returns
    ///
    /// The 64-character lowercase hex representation of the SHA-256 hash of `key`.
    ///
    /// # Examples
    ///
    /// ```
    /// let cache = Cache::new();
    /// let h = cache.hash_key("example");
    /// assert_eq!(h.len(), 64);
    /// ```
    fn hash_key(&self, key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(key);
        let hex = hasher.finalize();
        format!("{:x}", hex)
    }

    /// Remove the cached entry for `key` from the currently configured storage backend.
    ///
    /// Deletes the entry identified by `key` from either the in-memory store or the on-disk cache,
    /// depending on the cache's storage configuration.
    ///
    /// # Returns
    ///
    /// `true` if the entry was found and removed.
    ///
    /// # Errors
    ///
    /// - `CacheError::NotFound` if the entry does not exist or the file could not be removed on disk.
    /// - `CacheError::CacheDirUnavailable` if the system cache directory cannot be determined (disk storage).
    ///
    /// # Examples
    ///
    /// ```
    /// let mut cache = Cache::new().with_storage(Storage::Memory);
    /// cache.set("foo", "bar").unwrap();
    /// assert!(cache.delete("foo").unwrap());
    /// assert!(cache.get("foo").unwrap().is_none());
    /// ```
    #[allow(dead_code)]
    fn delete(&self, key: &str) -> CacheResult<bool> {
        let key_hash = self.hash_key(key);
        match self.storage {
            Storage::Memory => {
                if self.entries.remove(&key_hash).is_some() {
                    Ok(true)
                } else {
                    Err(CacheError::NotFound)
                }
            }
            Storage::Disk => {
                let cache_dir = dirs::cache_dir().ok_or(CacheError::CacheDirUnavailable)?;
                let path = cache_dir.join(CACHE_DIR).join(key_hash);
                remove_file(path).map_err(|_| CacheError::NotFound)?;
                Ok(true)
            }
        }
    }

    /// Checks whether a cache item is older than 24 hours.
    ///
    /// # Returns
    ///
    /// `true` if the item's `last_update` is more than 24 hours ago, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::{SystemTime, Duration};
    /// // create a cache and items for demonstration
    /// let cache = Cache::new();
    ///
    /// let fresh = CacheItem { value: "a".into(), last_update: SystemTime::now() };
    /// assert!(!cache.is_stale(&fresh));
    ///
    /// let stale = CacheItem { value: "b".into(), last_update: SystemTime::now() - Duration::from_secs(48 * 60 * 60) };
    /// assert!(cache.is_stale(&stale));
    /// ```
    fn is_stale(&self, item: &CacheItem) -> bool {
        item.last_update
            .elapsed()
            .map(|d| d > Duration::from_secs(24 * 60 * 60))
            .unwrap_or(false)
    }
}

impl Cacheable for CacheItem {
    /// Produce an owned `CacheItem` with the same `value` and `last_update`.
    ///
    /// # Examples
    ///
    /// ```
    /// let original = CacheItem { value: "v".into(), last_update: std::time::SystemTime::now() };
    /// let owned = original.to_cache_value();
    /// assert_eq!(owned.value, "v");
    /// ```
    fn to_cache_value(self) -> CacheItem {
        CacheItem {
            value: self.value,
            last_update: self.last_update,
        }
    }
}

impl Transport for Cache {
    /// Stores a cacheable value under the given key using the cache's configured storage backend.
    ///
    /// When the cache is configured for disk storage, the cache directory will be created if needed and the item is persisted as `timestamp|value`.
    ///
    /// # Returns
    ///
    /// `Ok(true)` if the value was stored successfully, `Err(CacheError)` on failure.
    ///
    /// # Examples
    ///
    /// ```
    /// let cache = Cache::new();
    /// cache.set("user:1", "Alice").unwrap();
    /// assert_eq!(cache.get("user:1").unwrap().as_deref(), Some("Alice"));
    /// ```
    fn set<V>(&self, key: &str, value: V) -> CacheResult<bool>
    where
        V: Cacheable,
    {
        let key_hash = self.hash_key(key);
        match self.storage {
            Storage::Memory => {
                self.entries.insert(key_hash, value.to_cache_value());
                Ok(true)
            }
            Storage::Disk => {
                let cache_dir = dirs::cache_dir().ok_or(CacheError::CacheDirUnavailable)?;
                let cache_item = value.to_cache_value();
                let path_string = cache_dir.join(CACHE_DIR).join(&key_hash);

                if let Some(parent) = path_string.parent() {
                    create_dir_all(parent).map_err(|_| CacheError::UknownError)?;
                }

                let timestamp = cache_item
                    .last_update
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                let content = format!("{}|{}", timestamp, cache_item.value);

                match write(&path_string, content) {
                    Ok(_) => Ok(true),
                    Err(err) => match err.kind() {
                        std::io::ErrorKind::NotFound => {
                            Err(CacheError::FileNotFound { path: path_string })
                        }
                        _ => Err(CacheError::UknownError),
                    },
                }
            }
        }
    }

    /// Retrieves a cached string for `key` if present and not expired.
    ///
    /// Returns `Ok(Some(value))` when a non-stale cached value is found, `Ok(None)` when the key
    /// is missing or the stored item is stale (it will be removed), and `Err(CacheError::CacheDirUnavailable)`
    /// if the disk cache is selected but the system cache directory cannot be determined.
    ///
    /// # Examples
    ///
    /// ```
    /// let cache = Cache::new();
    /// // nothing stored yet
    /// assert!(cache.get("missing").unwrap().is_none());
    ///
    /// let cache = cache.with_storage(Storage::Memory);
    /// cache.set("k", "v").unwrap();
    /// assert_eq!(cache.get("k").unwrap(), Some("v".to_string()));
    /// ```
    fn get(&self, key: &str) -> CacheResult<Option<String>> {
        debug!("Looking for {} in cache", key);
        let key_hash = self.hash_key(key);
        match self.storage {
            Storage::Memory => {
                if let Some(item) = self.entries.get(&key_hash) {
                    if self.is_stale(&item) {
                        drop(item);
                        self.entries.remove(&key_hash);
                        debug!("Item for {key} is stale, removed from cache.");
                        return Ok(None);
                    }
                    let val = Some(item.value.clone());
                    debug!("Found value for {key} in memory.");
                    debug!("{:?}", val);
                    Ok(val)
                } else {
                    Ok(None)
                }
            }
            Storage::Disk => {
                let cache_dir = dirs::cache_dir().ok_or(CacheError::CacheDirUnavailable)?;
                let content = match read_to_string(cache_dir.join(CACHE_DIR).join(&key_hash)) {
                    Ok(val) => val,
                    Err(e) => {
                        debug!("Cache not found on disk: {e}");
                        return Ok(None);
                    }
                };

                if let Some((timestamp_str, value)) = content.split_once('|') {
                    if let Ok(timestamp) = timestamp_str.parse::<u64>() {
                        let last_update = SystemTime::UNIX_EPOCH + Duration::from_secs(timestamp);
                        let item = CacheItem {
                            value: value.to_string(),
                            last_update,
                        };

                        if self.is_stale(&item) {
                            debug!("Item for {key} is stale on disk.");
                            let _ = remove_file(cache_dir.join(CACHE_DIR).join(&key_hash));
                            return Ok(None);
                        }

                        debug!("Found value for {key} in disk.");
                        return Ok(Some(value.to_string()));
                    }
                }

                Ok(None)
            }
        }
    }

    /// Removes stale cache entries according to the configured storage backend.
    ///
    /// For in-memory storage this evicts entries older than 24 hours from the internal map.
    /// For disk storage this removes files whose stored timestamp is older than 24 hours.
    ///
    /// # Examples
    ///
    /// ```
    /// let cache = Cache::new();
    /// // prune returns Ok(true) when the operation completes successfully
    /// assert!(cache.prune().unwrap());
    /// ```
    fn prune(&self) -> CacheResult<bool> {
        match self.storage {
            Storage::Memory => {
                let stale_items: Vec<_> = self
                    .entries
                    .iter()
                    .filter(|item| self.is_stale(&item.value()))
                    .map(|item| item.key().clone())
                    .collect();

                for item in stale_items {
                    let _ = self.entries.remove(&item);
                }

                Ok(true)
            }
            Storage::Disk => {
                let cache_dir = dirs::cache_dir().ok_or(CacheError::CacheDirUnavailable)?;
                let cache_path = cache_dir.join(CACHE_DIR);

                if !cache_path.exists() {
                    return Ok(true);
                }

                let entries = std::fs::read_dir(cache_path).map_err(|_| CacheError::UknownError)?;

                for entry in entries.flatten() {
                    if let Ok(content) = read_to_string(entry.path()) {
                        if let Some((timestamp_str, _)) = content.split_once('|') {
                            if let Ok(timestamp) = timestamp_str.parse::<u64>() {
                                let last_update =
                                    SystemTime::UNIX_EPOCH + Duration::from_secs(timestamp);
                                let item = CacheItem {
                                    value: String::new(),
                                    last_update,
                                };

                                if self.is_stale(&item) {
                                    let _ = remove_file(entry.path());
                                }
                            }
                        }
                    }
                }

                Ok(true)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::time::{Duration, SystemTime};

    use crate::utils::cache::{Cache, CacheItem, Storage, Transport};

    #[test]
    fn test_in_memory_cache() {
        let store = Cache::new();
        store
            .set("https://rb.gy/4wqwzf", "https://stanleymasinde.com")
            .unwrap();
        let result = store.get("https://rb.gy/4wqwzf");

        assert_eq!(
            result.unwrap().unwrap(),
            "https://stanleymasinde.com".to_string()
        );

        store.delete("https://rb.gy/4wqwzf").unwrap();

        assert!(store.get("https://rb.gy/4wqwzf").unwrap().is_none());
    }

    #[test]
    fn test_in_disk_cache() {
        let store = Cache::new().with_storage(Storage::Disk);
        store
            .set("https://rb.gy/4wqwzf", "https://stanleymasinde.com")
            .unwrap();
        let result = store.get("https://rb.gy/4wqwzf");

        assert_eq!(
            result.unwrap(),
            Some("https://stanleymasinde.com".to_string())
        );

        store.delete("https://rb.gy/4wqwzf").unwrap();

        assert!(store.get("https://rb.gy/4wqwzf").unwrap().is_none());
    }

    #[test]
    fn test_prune_cache() {
        let store = Cache::new();
        let key = "https://shortl.ink/4wqwzf";
        let value = "https://stanleymasinde.com";

        let new_item = CacheItem {
            value: value.to_string(),
            last_update: SystemTime::now() - Duration::from_secs(48 * 60 * 60),
        };

        {
            store.set(key, new_item).unwrap();

            let stored_link = store.get(key).unwrap();

            assert!(stored_link.is_none());
        }
    }

    #[test]
    fn test_stale_check_on_get() {
        let store = Cache::new();
        let key = "https://example.com/stale";
        let value = "https://destination.com";

        let stale_item = CacheItem {
            value: value.to_string(),
            last_update: SystemTime::now() - Duration::from_secs(48 * 60 * 60),
        };

        store.entries.insert(store.hash_key(key), stale_item);

        let result = store.get(key).unwrap();
        assert!(result.is_none());
    }
}