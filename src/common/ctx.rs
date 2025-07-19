use std::any::Any;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicIsize, AtomicUsize, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;
use pin_project_lite::pin_project;
use crate::{AsBytes};

#[derive(Debug,Default,Clone)]
pub struct Ctx {
    status: Arc<AtomicUsize>,
    subtask: Arc<AtomicIsize>,
    map:Arc<RwLock<HashMap<Vec<u8>,Box<dyn Any+Send+Sync>>>>
}
impl Ctx {
    #[allow(dead_code)]
    pub fn insert<K:AsBytes,V:Any+ Send+Sync>(&self,key:K,val:V)->Option<Box<dyn Any+Send+Sync>>{
        let key = key.as_byte().to_vec();
        self.ref_inner_mut(|map|{
            map.insert(key,Box::new(val))
        })
    }
    #[allow(dead_code)]
    pub fn remove<K:AsBytes,V:Any>(&self,key:K)->Option<V>{
        let key = key.as_byte();
        self.ref_inner_mut(|map|{
            if let Some(s) = map.get(key) {
                if s.downcast_ref::<V>().is_none() {
                    return None
                }
            }else{
                return None
            }
            if let Some(s) = map.remove(key){
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
        let key = key.as_byte();
        self.ref_inner(|map|{
            let opt = map.get(key);
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
        let key = key.as_byte();
        self.ref_inner_mut(|map|{
            let opt = map.get_mut(key);
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
    pub fn ref_inner<O>(&self, handle: impl FnOnce(&HashMap<Vec<u8>, Box<dyn Any+ Send+Sync>>) -> O) ->O{
        let reader = self.map.read().unwrap();
        handle(reader.deref())
    }
    #[allow(dead_code)]
    pub fn ref_inner_mut<O>(&self, handle: impl FnOnce(&mut HashMap<Vec<u8>, Box<dyn Any+ Send+Sync>>) -> O) ->O{
        let mut write = self.map.write().unwrap();
        handle(write.deref_mut())
    }
    #[allow(dead_code)]
    pub fn add_sub_task(&self,count:isize){
        self.subtask.fetch_add(count,Ordering::Relaxed);
    }
    #[allow(dead_code)]
    pub fn done_sub_task(&self){
        self.subtask.fetch_sub(1,Ordering::Relaxed);
    }
    // stop all task
    #[allow(dead_code)]
    pub fn stop(&self){
        self.status.fetch_add(1,Ordering::Relaxed);
    }
    #[allow(dead_code)]
    pub fn wait_stop_status(&self, timeout:Option<Duration>) ->CtxFut{
        let forever = timeout.is_none();
        let timeout = match timeout {
            None => tokio::time::sleep(Duration::from_millis(10)),
            Some(t) => tokio::time::sleep(t),
        };
        let status = self.status.clone();
        let subtask = self.subtask.clone();
        CtxFut{status,subtask,timeout,forever,check_subtask:false}
    }
    // if set timeout, and result is timeout, The subtask does not continue
    #[allow(dead_code)]
    pub fn wait_all_subtask_over(&self, timeout:Option<Duration>) ->CtxFut{
        let forever = timeout.is_none();
        let timeout = match timeout {
            None => tokio::time::sleep(Duration::from_millis(10)),
            Some(t) => tokio::time::sleep(t),
        };
        let status = self.status.clone();
        let subtask = self.subtask.clone();
        CtxFut{status,subtask,timeout,forever,check_subtask:true}
    }
    #[allow(dead_code)]
    pub async fn exec_future<Fut,Out>(self,future:Fut,timeout:Option<Duration>)->anyhow::Result<Out>
        where Fut:Future<Output=anyhow::Result<Out>>+Send+'static
    {
        self.add_sub_task(1);
        let result =  tokio::select! {
                x = future =>{
                    x
                }
                x = self.wait_stop_status(timeout) =>{
                    Err(x.into())
                }
            };
        self.done_sub_task();
        return result
    }
    #[allow(dead_code)]
    pub async fn call_timeout<F,Fut,Out>(self,lambda:F,timeout:Option<Duration>)->anyhow::Result<Out>
        where Fut:Future<Output=anyhow::Result<Out>>+Send+'static,
            F:FnOnce(Ctx)->Fut
    {
        let future = lambda(self.clone());
        self.exec_future(future,timeout).await
    }
    #[allow(dead_code)]
    pub async fn call<F,Fut,Out>(self,lambda:F)->anyhow::Result<Out>
        where Fut:Future<Output=anyhow::Result<Out>>+Send+'static,
              F:FnOnce(Ctx)->Fut
    {
        self.call_timeout(lambda,None).await
    }

}

pin_project! {
pub struct CtxFut{
    status: Arc<AtomicUsize>,
    subtask: Arc<AtomicIsize>,
    #[pin]
    timeout:tokio::time::Sleep,
    forever:bool,
    check_subtask:bool,
}
}

#[derive(Debug,Clone,Eq,PartialEq)]
pub enum CtxFutResult{
    Over,
    Timeout,
}
impl Display for CtxFutResult{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{:?}",self)
    }
}
impl std::error::Error for CtxFutResult {
}
impl Future for CtxFut {
    type Output = CtxFutResult;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut pro = self.project();

        match pro.timeout.as_mut().poll(cx){
            Poll::Ready(_) => {
                if !*pro.forever {
                    if *pro.check_subtask {
                        pro.status.fetch_add(1,Ordering::Relaxed);
                    }
                    return Poll::Ready(CtxFutResult::Timeout)
                }
            }
            Poll::Pending => return Poll::Pending,
        };
        if *pro.check_subtask {
            if pro.subtask.load(Ordering::Relaxed) <= 0 {

                return Poll::Ready(CtxFutResult::Over)
            }
        }else{
            if pro.status.load(Ordering::Relaxed) > 0 {
                return Poll::Ready(CtxFutResult::Over)
            }
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
                x = c.wait_stop_status(None) =>{
                    println!("cancel:{:?}",x);
                }
            };
        });
        tokio::time::sleep(Duration::from_secs(5)).await;
        ctx.stop();
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    #[tokio::test]
    async fn test_call(){
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
        ctx.wait_all_subtask_over(Some(Duration::from_secs(3))).await;
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}