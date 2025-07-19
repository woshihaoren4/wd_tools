use crate::AsBytes;
use std::vec::IntoIter;

pub struct ByteMap<V> {
    root: Node<V>,
}

pub struct Node<V> {
    key: u8,
    data: Option<V>,
    next: Vec<Node<V>>,
}

impl<V> Node<V> {
    pub fn default(key: u8) -> Self {
        Node {
            key,
            data: None,
            next: vec![],
        }
    }
    pub fn insert(&mut self, mut keys: IntoIter<u8>, value: V) {
        let next = if let Some(i) = keys.next() {
            i
        } else {
            self.data = Some(value);
            return;
        };
        for i in self.next.iter_mut() {
            if i.key == next {
                i.insert(keys, value);
                return;
            }
        }
        let mut node = Node::default(next);
        node.insert(keys, value);
        self.next.push(node);
    }

    // 完全匹配其中的一项
    pub fn get<'a, I>(&self, mut keys: I) -> Option<&V>
    where
        I: Iterator<Item = &'a u8>,
    {
        let next = if let Some(i) = keys.next() {
            i
        } else {
            if let Some(ref s) = self.data {
                return Some(s);
            }
            return None;
        };
        for i in self.next.iter() {
            if &i.key == next {
                return i.get(keys);
            }
        }
        return None;
    }

    // 前缀匹配一项，即取最左前缀子集
    pub fn match_first<'a, I: Iterator<Item = &'a u8>>(&self, mut keys: I) -> Option<&V> {
        if let Some(ref s) = self.data {
            return Some(s);
        }
        let next = if let Some(i) = keys.next() {
            i
        } else {
            return None;
        };
        for i in self.next.iter() {
            if &i.key == next {
                return i.match_first(keys);
            }
        }
        return None;
    }

    // 匹配所有子集
    pub fn match_all<'a, I: Iterator<Item = &'a u8>>(
        &'a self,
        mut keys: I,
        mut val: Vec<&'a V>,
    ) -> Vec<&'a V> {
        if let Some(ref s) = self.data {
            val.push(s);
        }
        let next = if let Some(i) = keys.next() {
            i
        } else {
            return val;
        };
        for i in self.next.iter() {
            if &i.key == next {
                return i.match_all(keys, val);
            }
        }
        return val;
    }

    // 不清除节点，只清理数据
    pub fn remove<'a, I: Iterator<Item = &'a u8>>(&mut self, mut keys: I) -> Option<V> {
        let next = if let Some(i) = keys.next() {
            i
        } else {
            if self.data.is_none() {
                return None;
            }
            return std::mem::take(&mut self.data);
        };
        for i in self.next.iter_mut() {
            if &i.key == next {
                return i.remove(keys);
            }
        }
        return None;
    }
}

impl<V> ByteMap<V> {
    pub fn new() -> Self {
        ByteMap {
            root: Node::default(0),
        }
    }

    pub fn insert<K: AsBytes>(&mut self, key: &K, value: V) {
        let keys = key.as_byte().to_vec();
        self.root.insert(keys.into_iter(), value);
    }

    pub fn get<K: AsBytes>(&self, key: K) -> Option<&V> {
        let keys = key.as_byte();
        self.root.get(keys.iter())
    }

    pub fn match_first<K: AsBytes>(&self, keys: K) -> Option<&V> {
        let keys = keys.as_byte();
        return self.root.match_first(keys.iter());
    }

    // 匹配所有子集
    pub fn match_all<'a, K: AsBytes>(&'a self, keys: &'a K) -> Vec<&'a V> {
        let path = keys.as_byte();
        let vals = vec![];
        return self.root.match_all(path.iter(), vals);
    }

    pub fn remove<K: AsBytes>(&mut self, key: K) -> Option<V> {
        let keys = key.as_byte();
        self.root.remove(keys.iter())
    }
}

#[cfg(test)]
mod test {
    use crate::coll::byte_map::ByteMap;
    use std::collections::BTreeMap;

    #[test]
    fn byte_map_crud_test() {
        let key = vec![1, 2, 3];
        let value = 123;

        let mut map = ByteMap::new();
        map.insert(&key, value);

        let val = map.get(&key);
        assert_eq!(val.unwrap(), &value);

        let val = map.remove(&key);
        assert_eq!(val.unwrap(), value);

        let val = map.get(&key);
        assert_eq!(val, None);

        map.insert(&key, value);
        let key2 = vec![1, 2, 3, 0, 255];
        let value2 = 255;
        map.insert(&key2, value2);

        let val = map.match_first(&key2);
        assert_eq!(val.unwrap(), &value);

        let mut val = map.match_all(&key2);
        assert_eq!(val.remove(0), &value);
        assert_eq!(val.remove(0), &value2);
    }

    #[test]
    fn byte_map_chinese() {
        let mut map = ByteMap::new();
        map.insert(&("你好".chars().collect::<Vec<char>>()), "你好");
        map.insert(&("hello".chars().collect::<Vec<char>>()), "hello");
        map.insert(&("123".chars().collect::<Vec<char>>()), "123");

        let target = "飞流之下，123，hello，你好".chars().collect::<Vec<char>>();

        for i in 0..target.len() {
            if let Some(s) = map.match_first(&target[i..]) {
                println!("match ok ---> {}", s);
            }
        }
    }

    #[test]
    fn byte_map_batch() {
        let target_byte_map = false;
        // let max = vec![10_0000,50_0000,100_0000,500_0000,1000_0000];
        let max = vec![100_0000];
        for i in max {
            let mut map = ByteMap::new();
            let mut hash_map = BTreeMap::new();
            let start_time = std::time::Instant::now();
            for i in 0..i {
                if target_byte_map {
                    map.insert(&i, i);
                } else {
                    hash_map.insert(i, i);
                }
            }
            // println!("insert use time:{}ms",start_time.elapsed().as_millis());
            for i in 0..i {
                let _val = if target_byte_map {
                    map.get(&i).unwrap()
                } else {
                    hash_map.get(&i).unwrap()
                };
                // assert_eq!(*val,i)
            }
            let use_time = start_time.elapsed().as_millis();

            println!("--> size[{i}], use time:{use_time}ms")
        }
    }
}
