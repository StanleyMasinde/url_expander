use std::{
    fs::{create_dir_all, read_to_string, remove_file, write},
    path::{Path, PathBuf},
};

static CACHE_DIR: &str = "url_expander";

use dashmap::DashMap;
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Error, Debug)]
enum CacheError {
    #[error("Item not found in cache.")]
    NotFound,

    #[error("Path not found.")]
    FileNotFound { path: PathBuf },

    #[error("An unknown error occoured.")]
    UknownError,
}

enum Storage {
    Memory,
    Disk,
}

type CacheResult<T> = Result<T, CacheError>;

trait Transport {
    fn set(&self, key: &str, value: String) -> CacheResult<bool>;
    fn get(&self, key: &str) -> CacheResult<Option<String>>;
    fn delete(&self, key: &str) -> CacheResult<bool>;
}

struct Cache {
    entries: DashMap<String, String>,
    storage: Storage,
}

impl Cache {
    fn new() -> Self {
        Self {
            entries: DashMap::new(),
            storage: Storage::Memory,
        }
    }

    fn with_storage(mut self, store: Storage) -> Self {
        self.storage = store;
        self
    }

    fn hash_key(&self, key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(key);
        let hex = hasher.finalize();
        format!("{:x}", hex)
    }
}

impl Transport for Cache {
    fn set(&self, key: &str, value: String) -> CacheResult<bool> {
        match self.storage {
            Storage::Memory => {
                self.entries.insert(key.into(), value);
                Ok(true)
            }
            Storage::Disk => {
                let cache_dir = dirs::cache_dir().unwrap();
                let key_hash = self.hash_key(key);
                let path_string = cache_dir.join(CACHE_DIR).join(key_hash);
                // Create parent directories if they don't exist
                if let Some(parent) = path_string.parent() {
                    create_dir_all(parent).unwrap();
                }
                match write(&path_string, value) {
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
        match self.storage {
            Storage::Memory => {
                let val = self.entries.get(key).map(|v| v.clone());
                Ok(val)
            }
            Storage::Disk => {
                let cache_dir = dirs::cache_dir().unwrap();
                let key_hash = self.hash_key(key);
                let content = match read_to_string(cache_dir.join(CACHE_DIR).join(key_hash)) {
                    Ok(val) => Some(val),
                    Err(_) => None,
                };
                Ok(content)
            }
        }
    }

    fn delete(&self, key: &str) -> CacheResult<bool> {
        match self.storage {
            Storage::Memory => {
                self.entries.remove(key).unwrap();
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

#[cfg(test)]
mod test {
    use crate::utils::cache::{Cache, Storage, Transport};

    #[test]
    fn test_in_memory_cache() {
        let store = Cache::new();
        store
            .set("https://rb.gy/4wqwzf", "https://stanleymasinde.com".into())
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
            .set("https://rb.gy/4wqwzf", "https://stanleymasinde.com".into())
            .unwrap();
        let result = store.get("https://rb.gy/4wqwzf");

        assert_eq!(
            result.unwrap(),
            Some("https://stanleymasinde.com".to_string())
        );

        store.delete("https://rb.gy/4wqwzf").unwrap();

        assert!(store.get("https://rb.gy/4wqwzf").unwrap().is_none());
    }
}
