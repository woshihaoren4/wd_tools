mod channel;
mod sender;
mod receiver;
mod error;
mod waker;

use std::sync::Arc;
pub use channel::*;
pub use sender::*;
pub use receiver::*;
pub use error::*;


impl<T> Channel<T>
    where T:Unpin + 'static
{
    pub fn new(cap:usize)->(Sender<T>,Receiver<T>){
        let chan = Channel::with_capacity(cap);
        let chan = Arc::new(chan);
        waker::ChannelWaker::start_check(chan.clone());
        (Sender::new(chan.clone()),Receiver::new(chan))
    }
}

#[cfg(test)]
mod test{
    use std::time::Duration;
    use crate::sync::WaitGroup;
    use super::*;

    #[tokio::test]
    async fn test_channel(){
        let (sender,receiver) = Channel::<usize>::new(1);

        let res = sender.try_send(1).await;
        assert_eq!(res,Ok(()),"first try send failed");
        let res = sender.try_send(1).await;
        assert_eq!(res,Err(ChannelError::FULL(1)),"second try send failed");

        let res = receiver.try_recv().await;
        assert_eq!(res,Ok(1),"first try recv failed");
        let res = receiver.try_recv().await;
        assert_eq!(res,Err(ChannelError::EMPTY),"first try recv failed");
    }

    #[tokio::test]
    async fn test_channel_wait(){
        let wg = WaitGroup::default();
        let (sender,receiver) = Channel::<usize>::new(10);
        let start_time = std::time::Instant::now();
        wg.defer(||async move{
            for i in 0..10_0000{
                sender.send(i).await.expect("发送失败");
                // println!("send success -> {}",i);
            }
        });
        wg.defer(||async move{
            for i in 0..10_0000{
                let res = receiver.recv().await.expect("接收失败");
                // println!("recv {} success -> {}",i,res);
            }
        });
        wg.wait().await;
        println!("user time ===>{}ms",start_time.elapsed().as_millis())
    }
}