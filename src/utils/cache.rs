use std::{
    borrow::Borrow,
    fs::{self, read, read_to_string, remove_file, write},
    hash::Hash,
    str::from_utf8,
};

static CACHE_DIR: &str = "url_expander";

use dashmap::DashMap;

enum CacheError {
    NotFound,
}

enum Storage {
    Memory,
    Disk,
}

type CacheResult<T> = Result<T, CacheError>;

trait Transport {
    async fn set(&self, key: &str, value: String) -> CacheResult<bool>;
    async fn get(&self, key: &str) -> CacheResult<String>;
    async fn delete(&self, key: &str) -> CacheResult<bool>;
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
}

impl Transport for Cache {
    async fn set(&self, key: &str, value: String) -> CacheResult<bool> {
        match self.storage {
            Storage::Memory => {
                self.entries.insert(key.into(), value).unwrap();
                Ok(true)
            }
            Storage::Disk => {
                let cache_dir = dirs::cache_dir().unwrap();
                write(cache_dir.join(CACHE_DIR).join(key), value).unwrap();
                Ok(true)
            }
        }
    }

    async fn get(&self, key: &str) -> CacheResult<String> {
        match self.storage {
            Storage::Memory => {
                let val = self.entries.get(key).map(|v| v.clone()).unwrap();
                Ok(val)
            }
            Storage::Disk => {
                let cache_dir = dirs::cache_dir().unwrap();
                let content = read_to_string(cache_dir.join(CACHE_DIR).join(key)).unwrap();
                Ok(content)
            }
        }
    }

    async fn delete(&self, key: &str) -> CacheResult<bool> {
        match self.storage {
            Storage::Memory => {
                self.entries.remove(key).unwrap();
                Ok(true)
            }
            Storage::Disk => {
                let cache_dir = dirs::cache_dir().unwrap();
                remove_file(cache_dir.join(CACHE_DIR).join(key)).unwrap();
                Ok(true)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::utils::cache::{Cache, Transport};

    #[test]
    fn test_set_cache() {
        let store = Cache::new();
        store.set("https://rb.gy/4wqwzf", "https://stanleymasinde.com".into());
        let result = store.get("https://rb.gy/4wqwzf");

        assert_eq!(result, "https://stanleymasinde.com")
    }
}
