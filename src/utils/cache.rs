use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

use dashmap::DashMap;
use log::debug;
use sha2::{Digest, Sha256};
use tokio::fs::{create_dir_all, read_to_string, remove_file, write};

use crate::types::{
    CACHE_DIR, Cache, CacheError, CacheItem, CacheResult, Cacheable, Storage, Transport,
};

impl Cache {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
            storage: Storage::Memory,
        }
    }

    pub fn with_storage(mut self, store: Storage) -> Self {
        self.storage = store;
        self
    }

    fn hash_key(&self, key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(key);
        let hex = hasher.finalize();
        format!("{:x}", hex)
    }

    #[allow(dead_code)]
    async fn delete(&self, key: &str) -> CacheResult<bool> {
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
                remove_file(path).await.map_err(|_| CacheError::NotFound)?;
                Ok(true)
            }
        }
    }

    fn is_stale(&self, item: &CacheItem) -> bool {
        item.last_update
            .elapsed()
            .map(|d| d > Duration::from_secs(24 * 60 * 60))
            .unwrap_or(false)
    }
}

impl Cacheable for CacheItem {
    fn to_cache_value(self) -> CacheItem {
        CacheItem {
            value: self.value,
            last_update: self.last_update,
        }
    }
}

impl Transport for Cache {
    async fn set<V>(&self, key: &str, value: V) -> CacheResult<bool>
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
                    create_dir_all(parent)
                        .await
                        .map_err(|_| CacheError::UknownError)?;
                }

                let timestamp = cache_item
                    .last_update
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                let content = format!("{}|{}", timestamp, cache_item.value);

                match write(&path_string, content).await {
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

    async fn get(&self, key: &str) -> CacheResult<Option<String>> {
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
                let content = match read_to_string(cache_dir.join(CACHE_DIR).join(&key_hash)).await
                {
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
                        let _ = remove_file(cache_dir.join(CACHE_DIR).join(&key_hash)).await;
                        return Ok(None);
                    }

                    debug!("Found value for {key} in disk.");
                    return Ok(Some(value.to_string()));
                }

                Ok(None)
            }
        }
    }

    async fn prune(&self) -> CacheResult<bool> {
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
                    if let Ok(content) = read_to_string(entry.path()).await
                        && let Some((timestamp_str, _)) = content.split_once('|')
                        && let Ok(timestamp) = timestamp_str.parse::<u64>()
                    {
                        let last_update = SystemTime::UNIX_EPOCH + Duration::from_secs(timestamp);
                        let item = CacheItem {
                            value: String::new(),
                            last_update,
                        };

                        if self.is_stale(&item) {
                            let _ = remove_file(entry.path()).await;
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

    #[tokio::test]
    async fn test_in_memory_cache() {
        let store = Cache::new();
        store
            .set("https://rb.gy/4wqwzf", "https://stanleymasinde.com")
            .await
            .unwrap();
        let result = store.get("https://rb.gy/4wqwzf");

        assert_eq!(
            result.await.unwrap().unwrap(),
            "https://stanleymasinde.com".to_string()
        );

        store.delete("https://rb.gy/4wqwzf").await.unwrap();

        assert!(store.get("https://rb.gy/4wqwzf").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_in_disk_cache() {
        let store = Cache::new().with_storage(Storage::Disk);
        store
            .set("https://rb.gy/4wqwzf", "https://stanleymasinde.com")
            .await
            .unwrap();
        let result = store.get("https://rb.gy/4wqwzf");

        assert_eq!(
            result.await.unwrap(),
            Some("https://stanleymasinde.com".to_string())
        );

        store.delete("https://rb.gy/4wqwzf").await.unwrap();

        assert!(store.get("https://rb.gy/4wqwzf").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_prune_cache() {
        let store = Cache::new();
        let key = "https://shortl.ink/4wqwzf";
        let value = "https://stanleymasinde.com";

        let new_item = CacheItem {
            value: value.to_string(),
            last_update: SystemTime::now() - Duration::from_secs(48 * 60 * 60),
        };

        {
            store.set(key, new_item).await.unwrap();

            let stored_link = store.get(key).await.unwrap();

            assert!(stored_link.is_none());
        }
    }

    #[tokio::test]
    async fn test_stale_check_on_get() {
        let store = Cache::new();
        let key = "https://example.com/stale";
        let value = "https://destination.com";

        let stale_item = CacheItem {
            value: value.to_string(),
            last_update: SystemTime::now() - Duration::from_secs(48 * 60 * 60),
        };

        store.entries.insert(store.hash_key(key), stale_item);

        let result = store.get(key).await.unwrap();
        assert!(result.is_none());
    }
}
