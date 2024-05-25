use std::collections::VecDeque;
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::time::Duration;
use tokio::sync::Mutex;

#[async_trait::async_trait]
pub trait ObjFactor<T>:Send{
    async fn make(&self) -> Option<T>;
}

#[derive(Clone)]
pub struct ObjPool<T>{
    pub max:usize,
    pub idle:usize,
    pub have:Arc<AtomicIsize>,
    pub factory: Arc<dyn ObjFactor<T> +Sync+'static>,
    pub pool : Arc<Mutex<VecDeque<T>>>
}

impl<T:Debug> Debug for ObjPool<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"max:{} idle:{} have:{:?} pool:{:?}",self.max,self.idle,self.have,self.pool)
    }
}

impl<T> ObjPool<T>{
    pub fn new<F:ObjFactor<T>+Sync+'static>(max:usize,idle:usize,factory:F)->Self{
        let factory = Arc::new(factory);
        let have = Arc::new(AtomicIsize::default());
        let pool = Arc::new(Mutex::new(VecDeque::new()));
        Self{factory,have,max,idle,pool}
    }

    pub async fn defer<F,FUT,O>(&self, handle:F)->anyhow::Result<O>
    where F:FnOnce(&mut T)-> FUT + Send ,
          FUT:  Future<Output=anyhow::Result<O>> +Send ,
        O:Send,
    {
        for i in 0..100{
            let mut t = if let Some(t) = self.try_pop().await {
                t
            }else{
                let t = Self::backoff(i);
                tokio::time::sleep(Duration::from_millis(t)).await;
                continue
            };
            let fut = handle(&mut t);
            let res = fut.await;
            self.try_push(t).await;

            return res
        }
        Err(anyhow::anyhow!("System busy"))
    }

    pub async fn try_pop(&self)->Option<T>{
        let mut lock = self.pool.lock().await;
        if let Some(t) = lock.pop_front() {
            return Some(t)
        }
        //超过最大值
        if self.have.load(Ordering::Relaxed) >= self.max as isize {
            return None
        }
        //新建
        if let Some(t) = self.factory.make().await{
            self.have.fetch_add(1,Ordering::Relaxed);
            return Some(t)
        }
        return None
    }
    pub async fn try_push(&self,t:T){
        let mut lock = self.pool.lock().await;
        if lock.len() < self.idle {
            lock.push_back(t)
        }else{
            self.have.fetch_sub(1,Ordering::Relaxed);
        }
    }

    pub fn backoff(i:usize)->u64{
        return match i {
            _ if i >9 =>1000,
            _ if i >3 => 100,
            _ => 10,
        }

    }
}

#[async_trait::async_trait]
impl<T,F> ObjFactor<T> for F
where F:Fn()->T + Send +Sync,T:Send
{
    async fn make(&self) -> Option<T>{
        Some(self())
    }
}

#[cfg(test)]
mod test{
    use std::time::Duration;
    use crate::pool::ObjPool;

    //cargo test  pool::connect_pool::test::test_pool -- --nocapture
    #[tokio::test(flavor ="multi_thread", worker_threads = 4)]
    async fn test_pool(){
        let pool = ObjPool::new(10,2,||{
            1000u64
        });
        let start_time = std::time::Instant::now();
        for i in 0..100{
            let pool = pool.clone();
            tokio::spawn(async move {
                pool.defer(|t:& mut u64|  {
                    let t = *t;
                    async move{
                        println!("--->{}",i);
                        tokio::time::sleep(Duration::from_millis(t)).await;
                        return Ok(());
                    }
                }).await.unwrap();
            });
        }
        tokio::time::sleep(Duration::from_secs(20)).await;
        println!("pool=>{:?}",pool);

    }
}