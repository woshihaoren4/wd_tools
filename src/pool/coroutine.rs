use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicIsize, AtomicUsize, Ordering};

#[derive(Debug,Clone)]
pub struct ParallelPool {
    parallel_max: usize,
    workers : Arc<AtomicUsize>
}

impl ParallelPool {
    pub fn new(parallel:usize)->Self{
        let workers = Arc::new(AtomicUsize::new(0));
        Self{parallel_max:parallel,workers }
    }

    pub fn try_launch<F>(&self,f:F)->Option<F>
        where F:Future<Output=()> + Send + 'static
    {
        if self.workers.load(Ordering::Relaxed) > self.parallel_max {
            return Some(f);
        }
        //乐观锁
        let count = self.workers.fetch_add(1, Ordering::Relaxed);
        if count >= self.parallel_max {
            self.workers.fetch_sub(1,Ordering::Relaxed);
            return Some(f);
        }
        let workers = self.workers.clone();
        tokio::spawn(async move {
            f.await;
            workers.fetch_sub(1,Ordering::Relaxed);
        });
        return None
    }

    pub async fn launch<F>(&self,mut f:F)
        where F:Future<Output=()> + Send + 'static
    {
        loop {
            f = if let Some(s) = self.try_launch(f) {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                s
            }else{
                return;
            };
        }
    }
}

#[cfg(test)]
mod test{
    use crate::pool::coroutine::ParallelPool;
    use crate::sync::WaitGroup;

    #[tokio::test]
    async fn test_parallel_pool(){
        let pp = ParallelPool::new(3);
        let wg = WaitGroup::new(100);
        for _ in  0..10 {
            let wg = wg.clone();
            let pp = pp.clone();
            tokio::spawn(async move{
                for i in 0..10 {
                    let wg = wg.clone();
                    pp.launch(async move{
                        println!("---> start {}",i);
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        println!("---> end   {}",i);
                        wg.done();
                    }).await;
                }
            });
        }
        wg.wait().await;


    }
}