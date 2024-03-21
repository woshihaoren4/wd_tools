use std::fmt::{Debug, Formatter};
use std::ops::{Deref};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc,Mutex};

const COPY_LOCK_LENGTH:usize = 2;

/// 复制锁
/// 适用于多度少些的场景，比如配置
pub struct CopyLock<T> {
    wl : Mutex<()>,
    list:[Arc<T>; COPY_LOCK_LENGTH],
    list_status:[AtomicU32; COPY_LOCK_LENGTH],
    index: AtomicUsize,

}

impl<T> CopyLock<T> {
    pub fn new(default: T) -> CopyLock<T> {
        let wl = Mutex::new(());
        let raw = Arc::new(default);
        let list = if let Ok(o) = (0..COPY_LOCK_LENGTH).map(|_|raw.clone()).collect::<Vec<_>>().try_into() {
            o
        }else{
            panic!("CopyLock.new ")
        };
        let list_status = (0..COPY_LOCK_LENGTH).map(|_|AtomicU32::new(0)).collect::<Vec<_>>().try_into().unwrap();
        let index = AtomicUsize::new(0);
        CopyLock {
            wl,list,index,list_status
        }
    }

    pub fn share(&self) -> Arc<T> {
        let index = self.index.load(Ordering::Relaxed);
        self.list_status[index].fetch_add(1,Ordering::SeqCst);
        let s = self.list[index].clone();
        self.list_status[index].fetch_sub (1,Ordering::SeqCst);
        return s
    }

    pub fn update<F>(&self, function: F)
    where
        F: FnOnce(Arc<T>) -> T,
    {
        let lock = self.wl.lock().expect("CopyLock.update lock error");
        let old_val = self.share();
        let new_val = Arc::new(function(old_val));
        let next_index = self.get_next_index();

        while self.list_status[next_index].load(Ordering::SeqCst) != 0 {

        }
        // unsafe {
        //     let arc = &mut *(&self.list as *const [Arc<T>;COPY_LOCK_LENGTH] as *mut [Arc<T>;COPY_LOCK_LENGTH]);
        //     arc[next_index] = new_val;
        // }
        self.index.store(next_index,Ordering::Release);
        unsafe {
            let pred_index = if next_index == 0 {
                COPY_LOCK_LENGTH - 1
            }else{
                next_index - 1
            };
            // let arc = &mut *(&self.list as *const [Arc<T>;COPY_LOCK_LENGTH] as *mut [Arc<T>;COPY_LOCK_LENGTH]);
            // arc[pred_index] = arc[next_index].clone();
        }

        drop(lock);
    }

    pub fn set(&self, t: T) {
        self.update(|_|t);
    }
    fn get_next_index(&self)->usize{
        (self.index.load(Ordering::Relaxed) + 1)% COPY_LOCK_LENGTH
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

impl<T:Default> Default for Acl<T> {
    fn default() -> Self {
        Acl::new(T::default())
    }
}

impl<T> Deref for Acl<T> {
    type Target = CopyLock<T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> Acl<T> {
    pub fn new(default:T) ->Acl<T>{
        let inner = Arc::new(CopyLock::new(default));
        Acl{inner}
    }
}

impl<T:Debug> Debug for Acl<T>{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{:?}",self.share())
    }
}

#[cfg(test)]
mod test{
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;
    use crate::sync::Acl;

    #[tokio::test(flavor ="multi_thread", worker_threads = 4)]
    async fn test_acl_update(){
        let acl = Acl::new(1usize);
        acl.update(|o|&*o+1);
        let arc = acl.share();
        assert_eq!(*arc,2);
        println!("success");
    }

    //cargo test --color=always -p wd_tools --lib sync::copy_lock::test::test_acl --no-fail-fast -- --exact  unstable-options --show-output --nocapture
    #[tokio::test(flavor ="multi_thread", worker_threads = 4)]
    async fn test_acl(){
        let index = Arc::new(AtomicUsize::new(1) );
        let wcp = Acl::new(1usize);
        for _ in 0..4{
            let acl = wcp.clone();
            tokio::spawn(async move{
                loop {
                    let a = acl.share();
                    if *a > 400000 {
                        break
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                println!("---> read over");
            });
        }
        for _ in 0..4{
            let acl = wcp.clone();
            let nb = index.clone();
            tokio::spawn(async move{
                for _ in 0..100000{
                    let _i = nb.fetch_add(1, Ordering::SeqCst);
                    acl.update(|o|&*o+1)
                }
                println!("---> write over");
            });
        }

        println!("success");
        tokio::time::sleep(Duration::from_secs(5)).await;
        println!("over");

    }
}