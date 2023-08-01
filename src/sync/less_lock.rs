use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};

/// 复制锁
/// 适用于多度少些的场景，比如配置
#[derive(Debug)]
pub struct CopyLock<T> {
    lock: Mutex<()>,
    async_lock: tokio::sync::Mutex<()>,
    index: AtomicUsize,
    inner: [RwLock<Arc<T>>; 2],
}

impl<T: Clone + Send + Sync> CopyLock<T> {
    pub fn new(t: T) -> CopyLock<T> {
        let t0 = RwLock::new(Arc::new(t.clone()));
        let t1 = RwLock::new(Arc::new(t));
        let inner = [t0, t1];
        let index = AtomicUsize::new(0);
        let lock = Mutex::new(());
        let async_lock = tokio::sync::Mutex::new(());
        CopyLock {
            lock,
            async_lock,
            index,
            inner,
        }
    }

    pub fn share(&self) -> Arc<T> {
        let rw = self.inner[self.index.load(Ordering::Relaxed)]
            .read()
            .expect("LessLock.arc read");
        rw.clone()
    }
    pub fn to_raw(&self) -> T {
        self.share().as_ref().clone()
    }
    pub fn update<F>(&self, function: F)
    where
        F: FnOnce(T) -> T,
    {
        let _unused = self.lock.lock().expect("LessLock.wlock update lock");
        let val = self.to_raw();
        let val = function(val);
        self.set(val);
    }
    pub async fn async_update<F, Fut>(&self, function: F) -> anyhow::Result<()>
    //true:放入 false:不放入
    where
        Fut: Future<Output = anyhow::Result<T>>,
        F: FnOnce(T) -> Fut + Send,
    {
        let _unused = self.async_lock.lock().await;
        let val = self.to_raw();
        let val = function(val).await?;
        self.set(val);
        Ok(())
    }

    fn set(&self, t: T) {
        let val = Arc::new(t);
        let index = self.get_next_index();
        let mut rl = self.inner[index]
            .write()
            .expect("LessLock.inner set writer");
        let mrl = rl.deref_mut();
        *mrl = val;
        drop(rl);
        self.index.store(index, Ordering::Relaxed);
    }
    fn get_next_index(&self) -> usize {
        if self.index.load(Ordering::Relaxed) == 0 {
            1
        } else {
            0
        }
    }
}


pub struct Acl<T>{
    inner: Arc<CopyLock<T>>
}

impl<T> Clone for Acl<T> {
    fn clone(&self) -> Self {
        Self{inner:self.inner.clone()}
    }
}

impl<T> Deref for Acl<T> {
    type Target = CopyLock<T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: Clone + Send + Sync> Acl<T> {
    pub fn new(t:T)->Acl<T>{
        let inner = Arc::new(CopyLock::new(t));
        Acl{inner}
    }
}