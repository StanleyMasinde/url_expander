use std::{
    fmt::Display,
    fs::{create_dir_all, read_to_string, remove_file, write},
    path::PathBuf,
    process,
    time::{Duration, Instant},
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

    #[error("An unknown error occoured.")]
    UknownError,
}

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
    last_update: Instant,
}

impl From<&str> for CacheItem {
    fn from(value: &str) -> Self {
        Self {
            value: value.to_string(),
            last_update: Instant::now(),
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
            last_update: Instant::now(),
        }
    }
}

pub(crate) struct Cache {
    entries: DashMap<String, CacheItem>,
    storage: Storage,
}

pub trait Cacheable {
    fn to_cache_value(self) -> CacheItem;
}

impl Cacheable for String {
    fn to_cache_value(self) -> CacheItem {
        CacheItem {
            value: self,
            last_update: Instant::now(),
        }
    }
}

impl Cacheable for &str {
    fn to_cache_value(self) -> CacheItem {
        CacheItem {
            value: self.to_string(),
            last_update: Instant::now(),
        }
    }
}

impl Cache {
    pub fn new() -> Self {
        Self {
            entries: DashMap::new(),
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
    fn delete(&self, key: &str) -> CacheResult<bool> {
        let key_hash = self.hash_key(key);
        match self.storage {
            Storage::Memory => {
                self.entries.remove(&key_hash).unwrap();
                Ok(true)
            }
            Storage::Disk => {
                let cache_dir = dirs::cache_dir().unwrap();
                remove_file(cache_dir.join(CACHE_DIR).join(self.hash_key(key))).unwrap();
                Ok(true)
            }
        }
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
                let cache_dir = match dirs::cache_dir() {
                    Some(dir) => dir,
                    None => {
                        process::exit(1);
                    }
                };
                let path_string = cache_dir.join(CACHE_DIR).join(key_hash);
                // Create parent directories if they don't exist
                if let Some(parent) = path_string.parent() {
                    create_dir_all(parent).unwrap();
                }
                match write(&path_string, value.to_cache_value().value) {
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

    fn get(&self, key: &str) -> CacheResult<Option<String>> {
        debug!("Looking for {} in cache", key);
        let key_hash = self.hash_key(key);
        match self.storage {
            Storage::Memory => {
                let val = self.entries.get(&key_hash).map(|v| v.value.clone());
                debug!("Found value for {key} in memory.");
                debug!("{:?}", val);
                Ok(val)
            }
            Storage::Disk => {
                let cache_dir = dirs::cache_dir().unwrap();
                let content = match read_to_string(cache_dir.join(CACHE_DIR).join(key_hash)) {
                    Ok(val) => Some(val),
                    Err(e) => {
                        debug!("Cache not found on disk: {e}");
                        None
                    }
                };

                if content.is_none() {
                    Err(CacheError::NotFound)
                } else {
                    debug!("Found value for {key} in disk.");
                    Ok(content)
                }
            }
        }
    }

    fn prune(&self) -> CacheResult<bool> {
        match self.storage {
            Storage::Memory => {
                let stale_items: Vec<_> = self
                    .entries
                    .iter()
                    .filter(|item| item.last_update.elapsed() > Duration::from_hours(24))
                    .map(|item| item.key().clone())
                    .collect();

                for item in stale_items {
                    self.entries.remove(&item).unwrap();
                }

                Ok(true)
            }
            Storage::Disk => Ok(true),
        }
    }
}

#[cfg(test)]
mod test {
    use std::{
        ops::Sub,
        time::{Duration, Instant},
    };

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

        assert!(store.get("https://rb.gy/4wqwzf").is_err());
    }

    #[test]
    fn test_prune_cache() {
        let store = Cache::new();
        let key = "https://shortl.ink/4wqwzf";
        let value = "https://stanleymasinde.com";

        let new_item = CacheItem {
            value: value.to_string(),
            last_update: Instant::now().sub(Duration::from_hours(48)),
        };

        {
            store.set(key, new_item).unwrap();

            let stored_link = store.get(key).unwrap().unwrap();

            assert_eq!(stored_link, value);
        }

        store.prune().unwrap();

        let stored_link = store.get(key).unwrap();

        assert!(stored_link.is_none());
    }
}
