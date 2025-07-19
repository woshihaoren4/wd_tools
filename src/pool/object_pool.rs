use std::collections::VecDeque;
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[async_trait::async_trait]
pub trait ObjFactor<T>: Send {
    async fn make(&self) -> Option<T>;
}

pub struct ObjPool<T> {
    pub max: usize,
    pub idle: usize,
    pub have: AtomicIsize,
    pub factory: Box<dyn ObjFactor<T> + Sync + 'static>,
    pub pool: Mutex<VecDeque<T>>,
    pub multi_try_new: bool,
}

pub struct Object<T> {
    pool: Arc<ObjPool<T>>,
    t: Option<T>,
}

impl<T> Drop for Object<T> {
    fn drop(&mut self) {
        let t = std::mem::take(&mut self.t);
        if let Some(t) = t {
            self.pool.try_push(t);
        }
    }
}

impl<T: Debug> Debug for ObjPool<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "max:{} idle:{} have:{:?} pool:{:?}",
            self.max, self.idle, self.have, self.pool
        )
    }
}

impl<T> ObjPool<T> {
    pub fn new<F: ObjFactor<T> + Sync + 'static>(max: usize, idle: usize, factory: F) -> Arc<Self> {
        let factory = Box::new(factory);
        let have = AtomicIsize::default();
        let pool = Mutex::new(VecDeque::new());
        let multi_try_new = false;
        Arc::new(Self {
            factory,
            have,
            max,
            idle,
            pool,
            multi_try_new,
        })
    }
    pub(crate) fn new_object(self: &Arc<ObjPool<T>>, t: T) -> Object<T> {
        Object {
            pool: self.clone(),
            t: Some(t),
        }
    }

    pub async fn defer<F, FUT, O>(self: &Arc<ObjPool<T>>, handle: F) -> anyhow::Result<O>
    where
        F: FnOnce(Object<T>) -> FUT + Send + Sync + 'static,
        FUT: Future<Output = anyhow::Result<O>> + Send,
        O: Send,
    {
        for i in 0..100 {
            let t = if let Some(t) = self.try_pop() {
                t
            } else if let Some(t) = self.try_new_obj().await {
                t
            } else if self.multi_try_new && !self.is_full() {
                return Err(anyhow::anyhow!("ObjPool: new object failed"));
            } else {
                let t = Self::backoff(i);
                tokio::time::sleep(Duration::from_millis(t)).await;
                continue;
            };
            let obj = self.new_object(t);

            let fut = handle(obj);
            let res = fut.await;
            return res;
        }
        Err(anyhow::anyhow!("ObjPool: System busy"))
    }

    pub(crate) fn try_pop(&self) -> Option<T> {
        let mut lock = self.pool.lock().unwrap();
        lock.pop_front()
    }
    pub(crate) async fn try_new_obj(&self) -> Option<T> {
        //超过最大值
        if self.have.load(Ordering::Relaxed) >= self.max as isize {
            return None;
        }
        //新建
        if let Some(t) = self.factory.make().await {
            self.have.fetch_add(1, Ordering::Relaxed);
            return Some(t);
        }
        return None;
    }
    pub(crate) fn try_push(&self, t: T) {
        let mut lock = self.pool.lock().unwrap();
        if lock.len() < self.idle {
            lock.push_back(t)
        } else {
            self.have.fetch_sub(1, Ordering::Relaxed);
        }
    }
    pub fn is_full(&self) -> bool {
        self.have.load(Ordering::Relaxed) >= self.max as isize
    }

    pub(crate) fn backoff(i: usize) -> u64 {
        return match i {
            _ if i > 9 => 1000,
            _ if i > 3 => 100,
            _ => 10,
        };
    }
}

impl<T> Deref for Object<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        if let Some(ref t) = self.t {
            return t;
        }
        panic!("Object.t is nil")
    }
}

impl<T> DerefMut for Object<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        if let Some(ref mut t) = self.t {
            return t;
        }
        panic!("Object.t is nil")
    }
}

#[async_trait::async_trait]
impl<T, F> ObjFactor<T> for F
where
    F: Fn() -> T + Send + Sync,
    T: Send,
{
    async fn make(&self) -> Option<T> {
        Some(self())
    }
}

#[cfg(test)]
mod test {
    use crate::pool::ObjPool;
    use std::ops::DerefMut;
    use std::time::Duration;

    //cargo test  pool::object_pool::test::test_pool -- --nocapture
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_pool<'a>() {
        let pool = ObjPool::new(10, 2, || 1000u64);
        for i in 0..100 {
            let pool = pool.clone();
            tokio::spawn(async move {
                pool.defer(move |mut obj| async move {
                    println!("--->{}", i);
                    tokio::time::sleep(Duration::from_millis(*obj.deref_mut())).await;
                    Ok(())
                })
                .await
                .unwrap();
            });
        }
        tokio::time::sleep(Duration::from_secs(15)).await;
        println!("pool=>{:?}", pool);
    }
}
