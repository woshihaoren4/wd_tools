use std::future::Future;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::Arc;
use tokio::sync::Notify;

#[derive(Default)]
pub struct WaitGroup {
    count: Arc<AtomicIsize>,
    notify: Arc<Notify>,
    // wait_fut: Option<Box<dyn Future<Output=()> + Send + 'static>>,
}

impl Clone for WaitGroup {
    fn clone(&self) -> Self {
        Self {
            count: self.count.clone(),
            notify: self.notify.clone(),
            // wait_fut: None,
        }
    }
}
unsafe impl Sync for WaitGroup {}
unsafe impl Send for WaitGroup {}

impl WaitGroup {
    pub fn new(count: isize) -> Self {
        Self {
            count: Arc::new(AtomicIsize::new(count)),
            notify: Arc::new(Notify::new()),
            // wait_fut: None,
        }
    }
    pub fn add(&self, count: isize) {
        self.count.fetch_add(count, Ordering::Release);
    }
    pub fn done(&self) {
        self.count.fetch_sub(1, Ordering::Release);
        self.notify.notify_waiters();
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
    pub async fn wait(&self) {
        loop {
            if self.count.load(Ordering::Acquire) <= 0 {
                return;
            }
            self.notify.notified().await;
        }
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
