use std::{
    fs::{create_dir_all, read_to_string, remove_file, write},
    sync::Arc,
    time::{Duration, SystemTime},
};

use dashmap::DashMap;
use log::debug;
use sha2::{Digest, Sha256};

use crate::types::{
    CACHE_DIR, Cache, CacheError, CacheItem, CacheResult, Cacheable, Storage, Transport,
};

impl Cache {
    /// Creates a new cache configured to use in-memory storage.
    ///
    /// # Examples
    ///
    /// ```
    /// let cache = Cache::new();
    /// cache.set("key", "value").unwrap();
    /// assert_eq!(cache.get("key").unwrap(), Some("value".to_string()));
    /// ```
    pub fn new() -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
            storage: Storage::Memory,
        }
    }

    /// Set the cache storage backend and return the modified `Cache` for chaining.
    ///
    /// This consumes `self`, assigns the provided `Storage` backend, and returns the updated cache.
    ///
    /// # Examples
    ///
    /// ```
    /// let cache = Cache::new().with_storage(Storage::Disk);
    /// ```
    pub fn with_storage(mut self, store: Storage) -> Self {
        self.storage = store;
        self
    }

    /// Compute the SHA-256 hash of `key` and return it as a lowercase hexadecimal string.
    ///
    /// Returns the lowercase hexadecimal SHA-256 digest of `key`.
    ///
    /// # Examples
    ///
    /// ```
    /// let cache = Cache::new();
    /// let digest = cache.hash_key("foo");
    /// assert_eq!(
    ///     digest,
    ///     "2c26b46b68ffc68ff99b453c1d30413413422d706483bfa0f98a5e886266e7ae"
    /// );
    /// ```
    fn hash_key(&self, key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(key);
        let hex = hasher.finalize();
        format!("{:x}", hex)
    }

    /// Remove the cache entry for the given original key.
    ///
    /// Attempts to delete the cached item identified by `key`. On success returns `true`.
    /// If no entry exists for the given key, returns `Err(CacheError::NotFound)`. When using
    /// disk-backed storage, returns `Err(CacheError::CacheDirUnavailable)` if the system cache
    /// directory cannot be resolved.
    ///
    /// # Returns
    ///
    /// `true` if the entry was removed, `Err(CacheError::NotFound)` if the entry does not exist,
    /// or `Err(CacheError::CacheDirUnavailable)` if the cache directory is unavailable for disk storage.
    ///
    /// # Examples
    ///
    /// ```
    /// let cache = Cache::new();
    /// // Deleting a missing key yields an error
    /// assert!(cache.delete("missing-key").is_err());
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

    /// Returns whether a cache item is older than 24 hours.
    ///
    /// `true` if the item's `last_update` is more than 24 hours ago, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::{SystemTime, Duration};
    ///
    /// let cache = Cache::new();
    /// let item = CacheItem {
    ///     value: String::from("v"),
    ///     last_update: SystemTime::now()
    ///         .checked_sub(Duration::from_secs(24 * 60 * 60 + 1))
    ///         .unwrap(),
    /// };
    /// assert!(cache.is_stale(&item));
    /// ```
    fn is_stale(&self, item: &CacheItem) -> bool {
        item.last_update
            .elapsed()
            .map(|d| d > Duration::from_secs(24 * 60 * 60))
            .unwrap_or(false)
    }
}

impl Cacheable for CacheItem {
    /// Produces a duplicate `CacheItem` with the same `value` and `last_update`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::SystemTime;
    ///
    /// let original = crate::utils::cache::CacheItem {
    ///     value: "v".to_string(),
    ///     last_update: SystemTime::now(),
    /// };
    /// let copy = original.to_cache_value();
    /// assert_eq!(copy.value, "v");
    /// assert_eq!(copy.last_update, original.last_update);
    /// ```
    ///
    /// @returns `CacheItem` with the same `value` and `last_update` as the receiver.
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
    /// On success returns `Ok(true)`; on failure returns a `CacheError`.
    ///
    /// # Examples
    ///
    /// ```
    /// let cache = Cache::new();
    /// let result = cache.set("example-key", "value").unwrap();
    /// assert!(result);
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

    /// Retrieves a cached string for `key`, removing and treating entries older than 24 hours as missing.
    ///
    /// On a hit, returns `Ok(Some(value))`. If the entry is absent, stale, or cannot be read or parsed from
    /// disk, returns `Ok(None)`. When a stale entry is observed it is removed from the underlying storage.
    ///
    /// # Examples
    ///
    /// ```
    /// let cache = Cache::new();
    /// cache.set("hello", "world").unwrap();
    /// assert_eq!(cache.get("hello").unwrap(), Some("world".to_string()));
    /// // After inserting a stale item (not shown), `get` would return `Ok(None)` and remove it.
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

                if let Some((timestamp_str, value)) = content.split_once('|')
                    && let Ok(timestamp) = timestamp_str.parse::<u64>()
                {
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

                Ok(None)
            }
        }
    }

    /// Removes stale cache entries from the configured storage backend.
    ///
    /// Performs a sweep of either in-memory entries or files on disk and deletes any items
    /// whose `last_update` is older than the staleness threshold.
    ///
    /// # Examples
    ///
    /// ```
    /// let cache = Cache::new();
    /// let result = cache.prune().unwrap();
    /// assert!(result);
    /// ```
    fn prune(&self) -> CacheResult<bool> {
        match self.storage {
            Storage::Memory => {
                let stale_items: Vec<_> = self
                    .entries
                    .iter()
                    .filter(|item| self.is_stale(item.value()))
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
                    if let Ok(content) = read_to_string(entry.path())
                        && let Some((timestamp_str, _)) = content.split_once('|')
                        && let Ok(timestamp) = timestamp_str.parse::<u64>()
                    {
                        let last_update = SystemTime::UNIX_EPOCH + Duration::from_secs(timestamp);
                        let item = CacheItem {
                            value: String::new(),
                            last_update,
                        };

                        if self.is_stale(&item) {
                            let _ = remove_file(entry.path());
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