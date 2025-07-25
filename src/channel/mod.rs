mod channel;
mod channel_split;
mod error;

pub use channel::*;
pub use channel_split::*;
pub use error::*;

#[cfg(test)]
mod test {
    use super::*;
    use crate::sync::WaitGroup;

    #[tokio::test]
    async fn test_channel() {
        let (sender, receiver) = Channel::<usize>::new(1);

        let res = sender.try_send(1);
        assert_eq!(res, Ok(()), "first try send failed");
        let res = sender.try_send(1);
        assert_eq!(res, Err(SendError::FULL(1)), "second try send failed");

        let res = receiver.try_recv();
        assert_eq!(res, Ok(1), "first try recv failed");
        let res = receiver.try_recv();
        assert_eq!(res, Err(RecvError::EMPTY), "first try recv failed");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_channel_wait() {
        let wg = WaitGroup::default();
        let (sender, receiver) = Channel::<usize>::new(100);
        let start_time = std::time::Instant::now();
        for _i in 0..100 {
            let sender = sender.clone();
            wg.defer(|| async move {
                for i in 0..10_0000 {
                    sender.send(i).await.expect("发送失败");
                    // println!("send success -> {}",i);
                }
            });
        }
        // sender.close();
        for _i in 0..100 {
            let receiver = receiver.clone();
            wg.defer(|| async move {
                for i in 0..10_0000 {
                    let res = receiver.recv().await;
                    match &res {
                        Ok(_) => {}
                        Err(e) => {
                            if let RecvError::CLOSED = e {
                                println!(" all recv -> {}", i);
                                return;
                            }
                        }
                    }
                    // println!("recv {} success -> {:?}",i,res);
                }
            });
        }
        wg.wait().await;
        println!("user time ===>{}ms", start_time.elapsed().as_millis())
    }
}
