use crate::AsBytes;
use pin_project_lite::pin_project;
use std::any::Any;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::future::Future;
use std::ops::{Add, Deref, DerefMut};
use std::pin::Pin;
use std::sync::atomic::{AtomicIsize, AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::sync::Notify;
use tokio::time::Sleep;

#[derive(Default)]
pub struct Ctx {
    status: Arc<AtomicUsize>,
    subtask: Arc<AtomicIsize>,
    map: Arc<RwLock<HashMap<Vec<u8>, Box<dyn Any + Send + Sync>>>>,
    notify: Arc<Notify>,
}
impl Clone for Ctx {
    fn clone(&self) -> Self {
        Self{
            status: self.status.clone(),
            subtask: self.subtask.clone(),
            map: self.map.clone(),
            notify: self.notify.clone(),
        }
    }
}
impl Ctx {
    #[allow(dead_code)]
    pub fn insert<K: AsBytes, V: Any + Send + Sync>(
        &self,
        key: K,
        val: V,
    ) -> Option<Box<dyn Any + Send + Sync>> {
        let key = key.as_byte().to_vec();
        self.ref_inner_mut(|map| map.insert(key, Box::new(val)))
    }
    #[allow(dead_code)]
    pub fn remove<K: AsBytes, V: Any>(&self, key: K) -> Option<V> {
        let key = key.as_byte();
        self.ref_inner_mut(|map| {
            if let Some(s) = map.get(key) {
                if s.downcast_ref::<V>().is_none() {
                    return None;
                }
            } else {
                return None;
            }
            if let Some(s) = map.remove(key) {
                let val = Box::into_raw(s) as *mut V;
                unsafe {
                    let a = Box::from_raw(val);
                    return Some(*a);
                }
            }
            None
        })
    }
    #[allow(dead_code)]
    pub fn ref_handle<K: AsBytes, V: Any, O>(
        &self,
        key: K,
        handle: impl FnOnce(Option<&V>) -> O,
    ) -> O {
        let key = key.as_byte();
        self.ref_inner(|map| {
            let opt = map.get(key);
            let res = match opt {
                None => None,
                Some(a) => a.downcast_ref::<V>(),
            };
            handle(res)
        })
    }
    #[allow(dead_code)]
    pub fn ref_handle_mut<K: AsBytes, V: Any, O>(
        &self,
        key: K,
        handle: impl FnOnce(Option<&mut V>) -> O,
    ) -> O {
        let key = key.as_byte();
        self.ref_inner_mut(|map| {
            let opt = map.get_mut(key);
            let res = match opt {
                None => None,
                Some(a) => a.downcast_mut::<V>(),
            };
            handle(res)
        })
    }
    #[allow(dead_code)]
    pub fn ref_inner<O>(
        &self,
        handle: impl FnOnce(&HashMap<Vec<u8>, Box<dyn Any + Send + Sync>>) -> O,
    ) -> O {
        let reader = self.map.read().unwrap();
        handle(reader.deref())
    }
    #[allow(dead_code)]
    pub fn ref_inner_mut<O>(
        &self,
        handle: impl FnOnce(&mut HashMap<Vec<u8>, Box<dyn Any + Send + Sync>>) -> O,
    ) -> O {
        let mut write = self.map.write().unwrap();
        handle(write.deref_mut())
    }
    #[allow(dead_code)]
    pub fn add_task(&self, count: isize) {
        self.subtask.fetch_add(count, Ordering::Release);
    }
    #[allow(dead_code)]
    pub fn done_task(&self) {
        self.subtask.fetch_sub(1, Ordering::Release);
        self.notify.notify_waiters();
    }
    // stop all task
    #[allow(dead_code)]
    pub fn stop(&self) {
        self.status.fetch_add(1, Ordering::Release);
        self.notify.notify_waiters();
    }
    #[allow(dead_code)]
    pub fn is_stop(&self) -> bool {
        self.status.load(Ordering::Acquire) > 0
    }
    #[allow(dead_code)]
    pub async fn wait_stop_status(&self) {
        loop {
            if self.is_stop() {
                return 
            }
            self.notify.notified().await;
        }
    }
    // if set timeout, and result is timeout, The subtask does not continue
    #[allow(dead_code)]
    pub async fn wait_all_subtask_over(&self) {
        loop {
            if self.subtask.load(Ordering::Acquire) <= 0 {
                return
            }
            self.notify.notified().await;
        }
    }
    #[allow(dead_code)]
    pub async fn exec_future<Fut, Out>(
        self,
        future: Fut,
        timeout: Option<Duration>,
    ) -> anyhow::Result<Out>
    where
        Fut: Future<Output = anyhow::Result<Out>> + Send + 'static,
    {
        self.add_task(1);
        let res = if let Some(d) = timeout { 
            match tokio::time::timeout(d,future).await{
                Ok(o) => o,
                Err(e) => Err(anyhow::Error::new(e))
            }
        }else{
            future.await
        };
        self.done_task();
        return res;
    }
    #[allow(dead_code)]
    pub async fn call_timeout<F, Fut, Out>(
        self,
        lambda: F,
        timeout: Option<Duration>,
    ) -> anyhow::Result<Out>
    where
        Fut: Future<Output = anyhow::Result<Out>> + Send + 'static,
        F: FnOnce(Ctx) -> Fut,
    {
        let future = lambda(self.clone());
        self.exec_future(future,timeout).await
    }
    #[allow(dead_code)]
    pub async fn call<F, Fut, Out>(self, lambda: F) -> anyhow::Result<Out>
    where
        Fut: Future<Output = anyhow::Result<Out>> + Send + 'static,
        F: FnOnce(Ctx) -> Fut,
    {
        self.call_timeout(lambda, None).await
    }
}

#[cfg(test)]
mod test {
    use crate::common::ctx::{Ctx};
    use std::time::Duration;

    #[tokio::test]
    async fn test_context() {
        let ctx = Ctx::default();
        ctx.insert("hello", true);
        let c = ctx.clone();
        tokio::spawn(async move {
            let x = c.remove::<_, bool>("hello").unwrap();
            assert_eq!(x, true);
        });

        tokio::time::sleep(Duration::from_secs(1)).await;
        let res = ctx.remove::<_, bool>("hello");
        assert_eq!(None, res)
    }
    #[tokio::test]
    async fn test_status() {
        let ctx = Ctx::default();
        let c = ctx.clone();
        tokio::spawn(async move {
            match tokio::time::timeout(Duration::from_secs(3),c.wait_stop_status()).await{
                Ok(_) => {
                    println!("over and quit");
                }
                Err(_) => {
                    println!("wait timeout");
                }
            }
        });
        tokio::time::sleep(Duration::from_secs(5)).await;
        ctx.stop();
        println!("start stop");
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    #[tokio::test]
    async fn test_call() {
        let ctx = Ctx::default();
        tokio::spawn(ctx.clone().call(|_x| async {
            tokio::time::sleep(Duration::from_secs(1)).await;
            println!("1-->success");
            Ok(())
        }));
        tokio::spawn(ctx.clone().call(|_x| async {
            tokio::time::sleep(Duration::from_secs(2)).await;
            println!("2-->success");
            Ok(())
        }));
        tokio::spawn(ctx.clone().call(|_x| async {
            tokio::time::sleep(Duration::from_secs(5)).await;
            println!("3-->success");
            Ok(())
        }));
        tokio::time::sleep(Duration::from_secs(1)).await;
        ctx.wait_all_subtask_over().await;
    }
}
