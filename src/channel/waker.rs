use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use futures::FutureExt;
use pin_project_lite::pin_project;
use crate::channel::Channel;
use tokio::time::Sleep;

pin_project!{
pub struct ChannelWaker<T>{
         #[pin]
    chan : Arc<Channel<T>>,
        #[pin]
    sleep: Sleep
    }
}

impl<T: Unpin + 'static> ChannelWaker<T> {
    pub fn start_check(chan : Arc<Channel<T>>){
        tokio::spawn(async move{
            ChannelWaker{chan,sleep:tokio::time::sleep(Duration::from_millis(1))}.await;
        });
    }

}

impl<T:Unpin> Future for ChannelWaker<T>{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        //if channel close ,return Ready
        if let Poll::Pending = this.sleep.poll_unpin(cx){
            return Poll::Pending
        }

        if  this.chan.recv_waker_buf().len() > 0 {
            let len = this.chan.len();
            if !this.chan.status() {
                this.chan.recv_waker_buf().wake(usize::MAX);
            }else if len > 0{
                this.chan.recv_waker_buf().wake(len);
            }
        }

        if  this.chan.send_waker_buf().len() > 0 {
            let len = this.chan.len();
            if !this.chan.status() {
                this.chan.send_waker_buf().wake(usize::MAX);
            }else if len < this.chan.cap(){
                this.chan.send_waker_buf().wake(this.chan.cap() - len);
            }
        }

        if this.chan.status() && this.chan.send_waker_buf().len() == 0 && this.chan.recv_waker_buf().len() == 0{
            return Poll::Ready(())
        }
        this.sleep.set(tokio::time::sleep(Duration::from_millis(1)));
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}