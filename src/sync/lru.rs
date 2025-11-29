use crate::AsBytes;
use std::collections::HashMap;
use std::hash::RandomState;
use std::{mem, ptr};

#[derive(Debug, Clone)]
pub struct LruNode {
    next: *mut LruNode,
    prev: *mut LruNode,
    key: Vec<u8>,
}
impl LruNode {
    fn new(key: Vec<u8>) -> LruNode {
        Self {
            next: ptr::null_mut(),
            prev: ptr::null_mut(),
            key,
        }
    }
}

struct Value<V> {
    value: V,
    node: *mut LruNode,
}
impl<V> Value<V> {
    fn new(value: V, node: *mut LruNode) -> Value<V> {
        Self { value, node }
    }
}
pub struct LruDoubleLink {
    head: *mut LruNode,
    tail: *mut LruNode,
}
impl LruDoubleLink {
    fn update(&mut self, node: *mut LruNode) {
        if self.head == node {
            return;
        }
        if self.head.is_null() {
            self.head = node;
            self.tail = node;
            return;
        }
        unsafe {
            if !(*node).prev.is_null() {
                let prev = (*node).prev;
                let next = (*node).next;
                (*prev).next = next;
                if next.is_null() {
                    if self.tail == node {
                        self.tail = prev
                    }
                } else {
                    (*next).prev = prev;
                }
            }
            (*node).next = self.head;
            if !(*node).prev.is_null() {
                (*node).prev = ptr::null_mut();
            }
            (*self.head).prev = node;
            self.head = node;
        }
    }
    fn remove_last(&mut self) -> Vec<u8> {
        if self.tail.is_null() {
            return Vec::new();
        }
        let node = self.tail;
        unsafe {
            if !(*node).prev.is_null() {
                self.tail = (*node).prev;
                (*self.tail).next = ptr::null_mut();
            } else {
                self.tail = ptr::null_mut();
                self.head = self.tail;
            }
            let node = Box::from_raw(node);
            return node.key;
        }
    }
}
impl Drop for LruDoubleLink {
    fn drop(&mut self) {
        while !self.tail.is_null() {
            let _ = self.remove_last();
        }
    }
}
pub struct LruCache<V> {
    cap: usize,
    map: HashMap<Vec<u8>, Value<V>, RandomState>,
    link: LruDoubleLink,
}

impl<V> LruCache<V> {
    pub fn new(cap: usize) -> Self {
        let map = HashMap::with_capacity(cap);
        let head = ptr::null_mut();
        let tail = ptr::null_mut();
        let link = LruDoubleLink { head, tail };
        Self { cap, map, link }
    }
    pub fn put<K: AsBytes>(&mut self, key: K, value: V) -> Option<V> {
        let key = key.as_byte();
        let (result, b) = if let Some(v) = self.map.get_mut(key) {
            let old = mem::replace(&mut v.value, value);
            (Some(old), v.node)
        } else {
            let b = Box::into_raw(Box::new(LruNode::new(key.to_vec())));
            self.map.insert(key.to_vec(), Value::new(value, b));
            (None, b)
        };
        self.link.update(b);
        self.check_cap();
        result
    }
    pub fn get<K: AsBytes>(&mut self, key: K) -> Option<&V> {
        let key = key.as_byte();
        let v = if let Some(v) = self.map.get(key) {
            v
        } else {
            return None;
        };
        self.link.update(v.node);
        Some(&v.value)
    }
    fn check_cap(&mut self) {
        if self.map.len() <= self.cap {
            return;
        }
        let key = self.link.remove_last();
        let opt = self.map.remove(&key);
        drop(opt);
    }
}

unsafe impl Send for LruDoubleLink {}
unsafe impl<V: Send> Send for Value<V> {}
unsafe impl Send for LruNode {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lru_basic() {
        let mut cache = LruCache::new(2);

        // 1. 插入 A, B
        cache.put("A", 1);
        cache.put("B", 2);
        assert_eq!(cache.map.len(), 2);

        // 2. 访问 A (此时 A 变为最近使用，B 变为最久未使用)
        assert_eq!(cache.get("A"), Some(&1));

        // 3. 插入 C (应该淘汰 B)
        cache.put("C", 3);

        // 验证 B 被淘汰
        assert_eq!(cache.get("B"), None);
        // 验证 A 还在
        assert_eq!(cache.get("A"), Some(&1));
        // 验证 C 还在
        assert_eq!(cache.get("C"), Some(&3));
    }

    #[test]
    fn test_update_existing_key() {
        let mut cache = LruCache::new(2);

        cache.put("A", 1);
        cache.put("B", 2);

        // 更新 A 的值 (A 变为最近使用)
        let old = cache.put("A", 100);
        assert_eq!(old, Some(1));
        assert_eq!(cache.get("A"), Some(&100));

        // 插入 C (应该淘汰 B，因为 A 刚被更新过)
        cache.put("C", 3);

        assert_eq!(cache.get("B"), None); // B 应该没了
        assert_eq!(cache.get("A"), Some(&100)); // A 应该还在
    }

    #[test]
    fn test_capacity_overflow_logic() {
        // 测试修复后的计数逻辑是否正确
        let mut cache = LruCache::new(1);

        cache.put("A", 1);
        // 重复 put 不应导致误删（如果使用 stock+=1 且不检查是否存在，这里就会出问题）
        cache.put("A", 2);
        cache.put("A", 3);

        assert_eq!(cache.map.len(), 1);
        assert_eq!(cache.get("A"), Some(&3));
    }

    #[test]
    fn test_memory_cleanup() {
        // 这个测试主要用于配合 Valgrind/Miri 检测内存泄漏
        // 在普通单元测试中，我们确保 drop 不会 panic
        {
            let mut cache = LruCache::new(5);
            cache.put("A", 1);
            cache.put("B", 2);
            cache.put("C", 3);
        } // 这里触发 Drop，如果逻辑有误可能会 Segfault
    }
}
