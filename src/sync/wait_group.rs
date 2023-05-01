use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::task::{Context, Poll};


#[derive(Debug,Default)]
pub struct WaitGroup{
    count:Arc<AtomicIsize>
}

impl Clone for WaitGroup {
    fn clone(&self) -> Self {
        Self{count:self.count.clone()}
    }
}


impl WaitGroup {
    pub fn new(count:isize)->Self{
        Self{count:Arc::new(AtomicIsize::new(count))}
    }
    pub fn add(&self,count:isize){
        self.count.fetch_add(count,Ordering::Relaxed);
    }
    pub fn done(&self){
        self.count.fetch_sub(1,Ordering::Relaxed);
    }
    pub fn defer<FN,FUT>(&self,function:FN)
    where FUT:Future<Output=()> + Send,
    FN:FnOnce()->FUT + Send + 'static
    {
        self.add(1);
        let wg = self.clone();
        tokio::spawn(async move {
            let output = function().await;
            wg.done();
            return output;
        });
    }
    pub async fn wait(self){
        while self.count.load(Ordering::Relaxed) != 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }
}

impl Future for WaitGroup {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let count = self.count.load(Ordering::Relaxed);
        if count <= 0{
            Poll::Ready(())
        }else{
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}


#[cfg(test)]
mod test{
    use crate::sync::WaitGroup;

    // 会在主协成中不断判断 当前线程是否为空 十分消耗资源
    #[tokio::test]
    async fn test_wait_group(){
        let wg = WaitGroup::new(10);
        for i in 0..10 {
            let wg = wg.clone();
            let t = std::time::Instant::now();
            tokio::spawn(async move{
                println!("[{}]---> start",i);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                println!("[{}]---> over {}",i,t.elapsed().as_secs());
                wg.done();
            });
        }
        wg.await;
        println!("over")
    }

    #[tokio::test]
    async fn test_wait_group_wait(){
        let wg = WaitGroup::default();
        for i in 0..10 {
            wg.defer(move ||async move{
                println!("[{}]---> start",i);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                println!("[{}]---> over",i);
            });
        }
        wg.wait().await;
        println!("over")
    }
}