use lru::LruCache;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};

pub struct AsyncLru<K, V> {
    group: usize,
    cache: Arc<Vec<Mutex<LruCache<K, Arc<V>>>>>,
}

impl<K, V> Clone for AsyncLru<K, V> {
    fn clone(&self) -> Self {
        let group = self.group;
        let cache = self.cache.clone();
        Self { group, cache }
    }
}
impl<K: Hash + Eq, V> Default for AsyncLru<K, V> {
    fn default() -> Self {
        AsyncLru::new(8, 1024)
    }
}

impl<K: Hash + Eq, V> AsyncLru<K, V> {
    pub fn new(group: usize, group_cap: usize) -> Self {
        let mut cache = Vec::with_capacity(group);
        for _ in 0..group {
            cache.push(Mutex::new(LruCache::new(
                NonZeroUsize::new(group_cap).unwrap(),
            )));
        }
        let cache = Arc::new(cache);
        Self { group, cache }
    }

    pub fn put(&self, k: K, v: V) {
        let gid = self.get_group_id(&k);
        let mut writer = self.cache[gid].lock().unwrap();
        writer.deref_mut().put(k, Arc::new(v));
    }

    pub fn get(&self, k: &K) -> Option<Arc<V>> {
        let gid = self.get_group_id(&k);
        let mut reader = self.cache[gid].lock().unwrap();
        reader.deref_mut().get(k).map(|x| x.clone())
    }

    fn get_group_id(&self, k: &K) -> usize {
        let mut hasher = DefaultHasher::new();
        k.hash(&mut hasher);
        let value = hasher.finish();
        value as usize % self.group
    }
}

#[cfg(test)]
mod test {
    use crate::sync::async_lru::AsyncLru;
    use crate::sync::WaitGroup;

    #[tokio::test]
    async fn test_async_lru() {
        let lru = AsyncLru::<String, usize>::new(8, 10);
        let wg = WaitGroup::default();
        let start_time = std::time::Instant::now();
        for _ in 0..4 {
            wg.defer_args1(
                move |lru| async move {
                    for i in 0..1000000 {
                        lru.put(format!("key_{}", i), i);
                    }
                },
                lru.clone(),
            );
        }
        for _ in 0..4 {
            wg.defer_args1(
                move |lru| async move {
                    for i in 0..1000000 {
                        let _ = lru.get(&format!("key_{}", i));
                    }
                },
                lru.clone(),
            );
        }
        wg.wait().await;
        let use_time = std::time::Instant::now() - start_time;
        println!("user_time: {}ms", use_time.as_millis())
    }
}
