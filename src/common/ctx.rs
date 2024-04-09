use std::any::Any;
use std::collections::HashMap;
use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;
use pin_project_lite::pin_project;
use crate::{AsBytes, Sha1};

#[derive(Debug,Default,Clone)]
pub struct Ctx {
    status: Arc<AtomicUsize>,
    map:Arc<RwLock<HashMap<String,Box<dyn Any+Send+Sync>>>>
}
impl Ctx {
    #[allow(dead_code)]
    pub fn insert<K:AsBytes,V:Any+ Send+Sync>(&self,key:K,val:V)->Option<Box<dyn Any+Send+Sync>>{
        let key = key.as_byte().sha1();
        self.ref_inner_mut(|map|{
            map.insert(key,Box::new(val))
        })
    }
    #[allow(dead_code)]
    pub fn remove<K:AsBytes,V:Any>(&self,key:K)->Option<V>{
        let key = key.as_byte().sha1();
        self.ref_inner_mut(|map|{
            if let Some(s) = map.get(key.as_str()) {
                if s.downcast_ref::<V>().is_none() {
                    return None
                }
            }else{
                return None
            }
            if let Some(s) = map.remove(key.as_str()){
                let val = Box::into_raw(s) as *mut V;
                unsafe {
                    let a = Box::from_raw(val);
                    return Some(*a)
                }
            }
            None
        })
    }
    #[allow(dead_code)]
    pub fn ref_handle<K:AsBytes,V:Any,O>(&self,key:K,handle:impl FnOnce(Option<&V>)->O)->O{
        let key = key.as_byte().sha1();
        self.ref_inner(|map|{
            let opt = map.get(key.as_str());
            let res = match opt {
                None => None,
                Some(a) => {
                    a.downcast_ref::<V>()
                }
            };
            handle(res)
        })
    }
    #[allow(dead_code)]
    pub fn ref_handle_mut<K:AsBytes,V:Any,O>(&self,key:K,handle:impl FnOnce(Option<&mut V>)->O)->O{
        let key = key.as_byte().sha1();
        self.ref_inner_mut(|map|{
            let opt = map.get_mut(key.as_str());
            let res = match opt {
                None => None,
                Some(a) => {
                    a.downcast_mut::<V>()
                }
            };
            handle(res)
        })
    }
    #[allow(dead_code)]
    pub fn ref_inner<O>(&self, handle: impl FnOnce(&HashMap<String, Box<dyn Any+ Send+Sync>>) -> O) ->O{
        let reader = self.map.read().unwrap();
        handle(reader.deref())
    }
    #[allow(dead_code)]
    pub fn ref_inner_mut<O>(&self, handle: impl FnOnce(&mut HashMap<String, Box<dyn Any+ Send+Sync>>) -> O) ->O{
        let mut write = self.map.write().unwrap();
        handle(write.deref_mut())
    }
    #[allow(dead_code)]
    pub fn stop(&self){
        self.status.fetch_add(1,Ordering::Relaxed);
    }
    #[allow(dead_code)]
    pub fn wait(&self,timeout:Option<Duration>)->CtxFut{
        let forever = timeout.is_none();
        let timeout = match timeout {
            None => tokio::time::sleep(Duration::from_millis(10)),
            Some(t) => tokio::time::sleep(t),
        };
        let status = self.status.clone();
        CtxFut{status,timeout,forever}
    }
}

pin_project! {
pub struct CtxFut{
    status: Arc<AtomicUsize>,
    #[pin]
    timeout:tokio::time::Sleep,
    forever:bool,
}
}

#[derive(Debug,Clone,Eq,PartialEq)]
pub enum CtxFutResult{
    Over,
    Timeout,
}


impl Future for CtxFut {
    type Output = CtxFutResult;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut pro = self.project();

        match pro.timeout.as_mut().poll(cx){
            Poll::Ready(_) => {
                if !*pro.forever {
                    return Poll::Ready(CtxFutResult::Timeout)
                }
            }
            Poll::Pending => return Poll::Pending,
        };
        if pro.status.load(Ordering::Relaxed) > 0 {
            return Poll::Ready(CtxFutResult::Over)
        }
        unsafe {
            let x = pro.timeout.get_unchecked_mut();
            *x = tokio::time::sleep(Duration::from_millis(10));
        }
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}




#[cfg(test)]
mod test{
    use std::time::Duration;
    use crate::common::ctx::{Ctx};

    #[tokio::test]
    async fn test_context(){
        let ctx = Ctx::default();
        ctx.insert("hello",true);
        let c = ctx.clone();
        tokio::spawn(async move {
            let x = c.remove::<_,bool>("hello").unwrap();
            assert_eq!(x,true);
        });

        tokio::time::sleep(Duration::from_secs(1)).await;
        let res = ctx.remove::<_,bool>("hello");
        assert_eq!(None,res)
    }
    #[tokio::test]
    async fn test_status(){
        let ctx = Ctx::default();
        let c = ctx.clone();
        tokio::spawn(async move {
            let sleep = tokio::time::sleep(Duration::from_secs(3));
            tokio::pin!(sleep);
            tokio::select! {
                _ = &mut sleep=>{
                    println!("task exec over");
                }
                x = c.wait(None) =>{
                    println!("cancel:{:?}",x);
                }
            };
        });
        tokio::time::sleep(Duration::from_secs(5)).await;
        ctx.stop();
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}