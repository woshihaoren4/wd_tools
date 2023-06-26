use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use pin_project_lite::pin_project;
use crate::channel::*;

#[derive(Debug)]
pub struct Sender<T>{
    chan : Arc<Channel<T>>,
}

impl<T: Unpin> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Sender{chan:self.chan.clone()}
    }
}

impl<T: Unpin > Sender<T> {
    pub(crate) fn new(chan : Arc<Channel<T>>)->Sender<T>{
        Sender{chan}
    }
    pub fn try_send(&self,t:T)-> SendFuture<T> {
        SendFuture::new(t, true, self.chan.clone())
    }
    pub fn send(&self,t:T)-> SendFuture<T> {
        SendFuture::new(t, false, self.chan.clone())
    }
}

pin_project!{
    #[derive(Debug)]
pub struct SendFuture<T>{
    once:bool,
    try_count : usize,
    #[pin]
    inner: Option<T>,
    chan : Arc<Channel<T>>,
}
}

impl<T> SendFuture<T> {
    pub fn new(t:T,once:bool,chan:Arc<Channel<T>>)-> SendFuture<T>{
        SendFuture {once,try_count:0,inner:Some(t),chan}
    }
}

// impl< T> Unpin for SendWait< T> {}
impl<T:Unpin > Future for SendFuture<T>{
    type Output = ChannelResult<(),T>;

    fn poll(self: Pin<&mut SendFuture<T>>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        if this.chan.len() >= this.chan.cap() { //如果已经满了
            if *this.once { //只是尝试发一次
                let t = this.inner.take().unwrap();
                return Poll::Ready(full_result(t));
            }
            if !this.chan.send_waker_buf().add(cx) {
                cx.waker().wake_by_ref();
            }
            return Poll::Pending
        }
        //有空余则尝试加入
        let err = match this.chan.try_send_lock() {
            None => {
                cx.waker().wake_by_ref();
                return Poll::Pending
            }
            Some(idx) => {
                let t = this.inner.take().unwrap();
                let result = this.chan.unsafe_send(t, idx);
                this.chan.unlock(idx);
                match result {
                    Ok(_) => {
                        //尝试唤醒接收waker
                        this.chan.recv_waker_buf().wake(1);
                        return Poll::Ready(send_success_result())
                    }
                    Err(err) => {err}
                }

            }
        };
        match err {
            ChannelError::EMPTY => {
                panic!("send msg, but buf is empty")
            }
            ChannelError::CLOSED(t) => {
                return Poll::Ready(close_result(t))
            }
            ChannelError::FULL(t) => {
                this.inner.replace(t);
                cx.waker().wake_by_ref();
                return Poll::Pending
            }
        }

    }
}
