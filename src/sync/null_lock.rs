use std::future::Future;
use std::ops::DerefMut;
use tokio::sync::RwLock;

///空心锁
///锁在创建时可以不放入内容，也可以在某个时刻销毁锁内的内容
pub struct NullLock<T> {
    inner: RwLock<Option<T>>,
}

impl<T> NullLock<T> {
    pub fn new() -> NullLock<T> {
        let inner = RwLock::new(None);
        Self { inner }
    }

    pub async fn init(&self, t: T) {
        let mut w = self.inner.write().await;
        *w.deref_mut() = Some(t);
    }

    pub async fn reset(&self) {
        let mut w = self.inner.write().await;
        *w.deref_mut() = None
    }

    pub async fn get(&self) -> Option<T>
    where
        T: Clone,
    {
        let r = self.inner.read().await;
        if let Some(s) = r.as_ref() {
            return Some(s.clone());
        }
        return None;
    }

    pub async fn get_unwrap(&self) -> T
    where
        T: Clone + Default,
    {
        let r = self.inner.read().await;
        if let Some(s) = r.as_ref() {
            return s.clone();
        }
        return T::default();
    }

    pub async fn map<Out, F>(&self, map: F) -> Option<Out>
    where
        F: FnOnce(&T) -> Out + Send,
    {
        let r = self.inner.read().await;
        if let Some(s) = r.as_ref() {
            return Some(map(s));
        }
        return None;
    }

    pub async fn map_mut<Out,F, M>(&self, map: F) -> anyhow::Result<Out>
        where
            M: Future<Output=anyhow::Result<Out>> + Send ,
            F: FnOnce(&mut T) -> M + Send
    {
        let mut w = self.inner.write().await;
        if w.is_none() {
            return Err(anyhow::anyhow!("NullLock need init"));
        }
        let arg= w.as_mut().unwrap();
        let m = map(arg);
        return m.await
    }
    pub fn map_raw(&self)->&RwLock<Option<T>>{
        &self.inner
    }
}
