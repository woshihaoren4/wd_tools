use super::LruCache;
use crate::{bytes_to_usize, AsBytes};
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};

pub struct AsyncLru<V> {
    group: usize,
    cache: Arc<Vec<Mutex<LruCache<V>>>>,
}

impl<V> Clone for AsyncLru<V> {
    fn clone(&self) -> Self {
        let group = self.group;
        let cache = self.cache.clone();
        Self { group, cache }
    }
}
impl<V> Default for AsyncLru<V> {
    fn default() -> Self {
        AsyncLru::new(8, 1024)
    }
}

impl<V> AsyncLru<V> {
    pub fn new(group: usize, group_cap: usize) -> Self {
        let mut cache = Vec::with_capacity(group);
        for _ in 0..group {
            cache.push(Mutex::new(LruCache::new(group_cap)));
        }
        let cache = Arc::new(cache);
        Self { group, cache }
    }

    pub fn put<K: AsBytes>(&self, k: K, v: V) {
        let gid = self.get_group_id(&k);
        let mut writer = self.cache[gid].lock().unwrap();
        writer.deref_mut().put(k, v);
    }

    pub fn get<K: AsBytes, Out>(&self, k: K, handle: impl FnOnce(Option<&V>) -> Out) -> Out {
        let gid = self.get_group_id(&k);
        let mut reader = self.cache[gid].lock().unwrap();
        let opt = reader.deref_mut().get(k);
        handle(opt)
    }

    pub fn get_mut<K: AsBytes, Out>(&self, k: K, handle: impl FnOnce(Option<&mut V>) -> Out) -> Out {
        let gid = self.get_group_id(&k);
        let mut reader = self.cache[gid].lock().unwrap();
        let opt = reader.deref_mut().get_mut(k);
        handle(opt)
    }

    fn get_group_id<K: AsBytes>(&self, k: K) -> usize {
        bytes_to_usize(k.as_byte()) % self.group
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::sync::WaitGroup;
    use std::time::Duration;

    #[tokio::test]
    async fn test_async_lru() {
        let lru = AsyncLru::<usize>::new(8, 10);
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
                        let _ = lru.get(&format!("key_{}", i), |_x| {});
                    }
                },
                lru.clone(),
            );
        }
        wg.wait().await;
        let use_time = std::time::Instant::now() - start_time;
        println!("user_time: {}ms", use_time.as_millis())
    }
    #[test]
    fn test_multithreaded_lru() {
        // 1. 创建一个分片 LRU，容量小一点以便测试淘汰
        // 4个分片，每个分片容量 10，总容量 40
        let lru = AsyncLru::<i32>::new(4, 20);

        let mut handles = vec![];

        // 2. 启动 8 个写入线程
        for i in 0..8 {
            let lru_clone = lru.clone();
            handles.push(std::thread::spawn(move || {
                for j in 0..100 {
                    // key 格式: "thread-0-key-1"
                    let key = format!("t{}-k{}", i, j);
                    lru_clone.put(key, j);
                }
            }));
        }

        // 3. 启动 4 个读取线程
        for i in 0..4 {
            let lru_clone = lru.clone();
            handles.push(std::thread::spawn(move || {
                // 稍微延时，让写入先跑一会儿
                std::thread::sleep(Duration::from_millis(1));
                for j in 0..100 {
                    let key = format!("t{}-k{}", i, j); // 读取对应写入线程的 Key

                    // 使用回调获取值，如果存在则克隆出来
                    let val = lru_clone.get(&key, |opt| {
                        opt.cloned() // Option<&i32> -> Option<i32>
                    });

                    // 这里的断言不能太严格，因为是 LRU，旧数据可能已经被淘汰了
                    // 只要程序不 Panic 且能读到部分数据即可
                    if let Some(v) = val {
                        assert_eq!(v, j);
                    }
                }
            }));
        }

        // 4. 等待所有线程结束
        for h in handles {
            h.join().unwrap();
        }

        // 5. 验证状态
        // 随便取一个最近肯定写入过的数据验证是否存在
        let val = lru.get("t0-k99", |opt| opt.cloned());
        assert_eq!(val, Some(99));

        println!("多线程测试通过！无死锁或 Panic。");
    }

    #[test]
    fn test_handle_closure_pattern() {
        let lru = AsyncLru::<String>::new(2, 5);
        lru.put("key1", "value1".to_string());

        // 测试 handle 闭包是否能正确返回值
        let result = lru.get("key1", |opt| match opt {
            Some(v) => format!("found: {}", v),
            None => "not found".to_string(),
        });

        assert_eq!(result, "found: value1");
    }
}
