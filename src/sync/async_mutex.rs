use std::cell::{UnsafeCell};
use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};

pub type Am<T> = AsyncMutex<T>;

#[derive(Debug)]
pub struct AsyncMutex<T>{
    data:UnsafeCell<T>,
    status:Arc<AtomicBool>
}

#[derive(Debug)]
pub struct AsyncMutexFut<T>{
    data: *mut T,
    status:Arc<AtomicBool>
}

#[derive(Debug)]
pub struct AsyncMutexGuard<T>{
    data: *mut T,
    status:Arc<AtomicBool>
}

unsafe impl<T> Send for AsyncMutex<T>{}
unsafe impl<T> Sync for AsyncMutex<T>{}

impl<T> AsyncMutex<T>{
    pub fn new(data:T)->Self{
        let data = UnsafeCell::new(data);
        let status = Arc::new(AtomicBool::default());
        Self{data,status}
    }
    pub fn lock(&self) ->AsyncMutexFut<T>{
        #[allow(unused_unsafe)]
        unsafe {
            let data = self.data.get();
            let status = self.status.clone();
            AsyncMutexFut{data,status}
        }
    }
    #[allow(dead_code)]
    pub fn synchronize(&self)->AsyncMutexGuard<T>{
        let mutex = self.lock();
        loop {
            if mutex.try_lock() {
                let data = mutex.data;
                let status = mutex.status.clone();
                return AsyncMutexGuard{data,status}
            }
        }
    }
}

unsafe impl<T> Send for AsyncMutexFut<T>{}

impl<T> Future for AsyncMutexFut<T>{
    type Output = AsyncMutexGuard<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.try_lock() {
            let data = self.data;
            let status = self.status.clone();
            return Poll::Ready(AsyncMutexGuard{data,status})
        }
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

impl<T> Drop for AsyncMutexGuard<T> {
    fn drop(&mut self) {
        self.status.store(false,Ordering::Relaxed);
    }
}

impl<T> AsyncMutexFut<T>{
    fn try_lock(&self)->bool{
        self.status.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_ok()
    }
}


impl<T> Deref for AsyncMutexGuard<T>{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data }
    }
}
impl<T> DerefMut for AsyncMutexGuard<T>{
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {&mut *self.data}
    }
}




#[cfg(test)]
mod test{
    use std::ops::{Deref, DerefMut};
    use std::sync::{Arc};
    use crate::sync::async_mutex::Am;
    use crate::sync::WaitGroup;

    //tokio mutex use time:    1711ms [10*10_0000]
    //wd_tools mutex use time:  118ms [10*10_0000]
    #[tokio::test(flavor ="multi_thread", worker_threads = 4)]
    pub async fn test_mutex(){
        // let am = Arc::new(tokio::sync::Mutex::new(0isize));
        let am = Arc::new(Am::new(0isize));

        let start = std::time::Instant::now();
        let wg = WaitGroup::default();
        for _ in 0..10{
            wg.defer_args1(|am|async move{
                for _ in 0..10_0000 {
                    let mut lock = am.lock().await;
                    *(lock.deref_mut()) +=1;
                }
            },am.clone());
        }

        wg.wait().await;

        // let guard = am.synchronize();
        let guard = am.lock().await;
        println!("use_time[{}ms]--->{}",start.elapsed().as_millis(),guard.deref());
        assert_eq!(*guard.deref(),100_0000isize)
    }

    //sta mutex use time:       178ms [10*10_0000]
    //wd_tools mutex use time:  360ms [10*10_0000]
    #[tokio::test(flavor ="multi_thread", worker_threads = 4)]
    pub async fn test_synchronize(){
        let am = Arc::new(std::sync::Mutex::new(0isize));
        // let am = Arc::new(Am::new(0isize));

        let start = std::time::Instant::now();
        let wg = WaitGroup::default();
        for _ in 0..10{
            wg.defer_args1(|am|async move{
                for _ in 0..10_0000 {
                    // let mut lock = am.synchronize();
                    let mut lock = am.lock().unwrap();
                    *(lock.deref_mut()) +=1;
                }
            },am.clone());
        }

        wg.wait().await;

        // let guard = am.synchronize();
        let guard = am.lock().unwrap();
        println!("use_time[{}ms]--->{}",start.elapsed().as_millis(),guard.deref());
        assert_eq!(*guard.deref(),100_0000isize)
    }
}