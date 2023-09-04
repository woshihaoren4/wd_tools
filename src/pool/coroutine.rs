use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;


#[derive(Debug)]
pub struct ParallelPool {
    parallel_max: usize,
    workers: Arc<AtomicUsize>,
}



impl Clone for ParallelPool {
    fn clone(&self) -> Self {
        ParallelPool{parallel_max:self.parallel_max.clone(),workers:self.workers.clone()}
    }
}
impl ParallelPool {
    pub fn new(parallel: usize) -> Self {
        let workers = Arc::new(AtomicUsize::new(0));
        Self {
            parallel_max: parallel,
            workers,
        }
    }

    pub fn try_launch<F>(&self, f: F) -> Option<F>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        if self.workers.load(Ordering::Relaxed) > self.parallel_max {
            return Some(f);
        }
        //乐观锁
        let count = self.workers.fetch_add(1, Ordering::Relaxed);
        if count >= self.parallel_max {
            self.workers.fetch_sub(1, Ordering::Relaxed);
            return Some(f);
        }
        let workers = self.workers.clone();
        tokio::spawn(async move {
            f.await;
            workers.fetch_sub(1, Ordering::Relaxed);
        });
        return None;
    }

    pub async fn launch<F>(&self, mut f: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        loop {
            f = if let Some(s) = self.try_launch(f) {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                s
            } else {
                return;
            };
        }
    }

    pub async fn wait_over(&self) {
        while self.workers.load(Ordering::Relaxed) != 0 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }
}

impl Future for ParallelPool  {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.workers.load(Ordering::Relaxed) == 0 {
            return Poll::Ready(())
        }
        let waker = cx.waker().clone();
        tokio::spawn(async move{
            tokio::time::sleep(Duration::from_millis(10)).await;
            waker.wake_by_ref();
        });
        return Poll::Pending
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;
    use crate::pool::coroutine::ParallelPool;
    //cargo test --color=always --package wd_tools --lib pool::coroutine::test::test_parallel_pool --no-fail-fast --  --exact  unstable-options --show-output --nocapture
    #[tokio::test]
    async fn test_parallel_pool() {
        let pp = ParallelPool::new(3);
        for i in 0..10 {
            pp.launch(async move{
               println!("task start --> {i}");
                tokio::time::sleep(Duration::from_secs(1)).await;
               println!("task end   --> {i}");
            }).await;
        }
        pp.await;
    }
}
