use std::ops::DerefMut;
use std::sync::{RwLock};

pub struct NullLock<T>{
    inner: RwLock<Option<T>>
}

impl<T> NullLock<T> {
    pub fn new()->NullLock<T>{
        let inner = RwLock::new(None);
        Self{inner}
    }

    pub fn init(&self,t:T){
        let mut w = self.inner.write().unwrap();
        *w.deref_mut() = Some(t);
    }

    pub fn drop(&self){
        let mut w = self.inner.write().unwrap();
        *w.deref_mut() = None
    }

    pub fn get(&self) -> Option<T>
        where T:Clone
    {
        let r = self.inner.read().unwrap();
        if let Some(s) = r.as_ref() {
            return Some(s.clone())
        }
        return None
    }

    pub fn get_unwrap(&self) -> T
        where T:Clone + Default
    {
        let r = self.inner.read().unwrap();
        if let Some(s) = r.as_ref() {
            return s.clone()
        }
        return T::default()
    }

    pub fn map<Out,F>(&self,ref_func:F)-> Option<Out>
    where F:FnOnce(&T)->Out
    {
        let r = self.inner.read().unwrap();
        if let Some(s) = r.as_ref() {
            return Some(ref_func(s))
        }
        return None
    }
}