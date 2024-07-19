use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use pin_project_lite::pin_project;
use tokio::time::Sleep;

#[derive(Debug, Default)]
pub struct WaitGroup {
    count: Arc<AtomicIsize>,
}

impl Clone for WaitGroup {
    fn clone(&self) -> Self {
        Self {
            count: self.count.clone(),
        }
    }
}

impl WaitGroup {
    pub fn new(count: isize) -> Self {
        Self {
            count: Arc::new(AtomicIsize::new(count)),
        }
    }
    pub fn add(&self, count: isize) {
        self.count.fetch_add(count, Ordering::Relaxed);
    }
    pub fn done(&self) {
        self.count.fetch_sub(1, Ordering::Relaxed);
    }
    pub fn defer<FN, FUT>(&self, function: FN)
    where
        FUT: Future<Output = ()> + Send,
        FN: FnOnce() -> FUT + Send + 'static,
    {
        self.add(1);
        let wg = self.clone();
        tokio::spawn(async move {
            let output = function().await;
            wg.done();
            return output;
        });
    }
    pub fn defer_args1<FN, FUT, ARGS1>(&self, function: FN, args1: ARGS1)
    where
        FUT: Future<Output = ()> + Send + 'static,
        FN: for<'a> FnOnce(ARGS1) -> FUT + Send,
        ARGS1: Send,
    {
        self.add(1);
        let wg = self.clone();
        let future = function(args1);
        tokio::spawn(async move {
            future.await;
            wg.done();
        });
    }
    pub fn wait(&self) -> WaitGroupFut {
        WaitGroupFut{
            sleep: tokio::time::sleep(Duration::from_millis(1)),
            count:self.count.clone()
        }
    }
}
pin_project! {
    pub struct WaitGroupFut {
        #[pin]
        sleep: Sleep,
        count: Arc<AtomicIsize>,
    }
}

impl Future for WaitGroupFut {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let mut sleep = this.sleep;
        let ok = sleep.as_mut().poll(cx).is_ready();
        if ok {
            if this.count.load(Ordering::Relaxed) <= 0 {
                return Poll::Ready(())
            }
            sleep.set(tokio::time::sleep(Duration::from_millis(1)));
            cx.waker().wake_by_ref();
        }
        return Poll::Pending;
    }
}

#[cfg(test)]
mod test {
    use crate::sync::WaitGroup;

    #[tokio::test]
    async fn test_wait_group() {
        let wg = WaitGroup::new(10);
        for i in 0..10 {
            let wg = wg.clone();
            let t = std::time::Instant::now();
            tokio::spawn(async move {
                println!("[{}]---> start", i);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                println!("[{}]---> over {}", i, t.elapsed().as_secs());
                wg.done();
            });
        }
        wg.wait().await;
        println!("over")
    }

    #[tokio::test]
    async fn test_wait_group_wait() {
        let wg = WaitGroup::default();
        for i in 0..10 {
            wg.defer(move || async move {
                println!("[{}]---> start", i);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                println!("[{}]---> over", i);
            });
        }
        wg.wait().await;
        println!("over")
    }
}
