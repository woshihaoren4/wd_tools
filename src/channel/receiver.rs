use crate::channel::*;
use pin_project_lite::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll::Ready;
use std::task::{Context, Poll};

#[derive(Debug)]
pub struct Receiver<T> {
    chan: Arc<Channel<T>>,
}

impl<T> Clone for Receiver<T> {
    fn clone(&self) -> Self {
        Self {
            chan: self.chan.clone(),
        }
    }
}

impl<T: Unpin> Receiver<T> {
    pub(crate) fn new(chan: Arc<Channel<T>>) -> Receiver<T> {
        Receiver { chan }
    }
    pub fn try_recv(&self) -> RecvFuture<T> {
        RecvFuture::new(true, self.chan.clone())
    }
    pub fn recv(&self) -> RecvFuture<T> {
        RecvFuture::new(false, self.chan.clone())
    }
    pub fn close(&self) {
        self.chan.close();
    }
}

pin_project! {
    #[derive(Debug)]
    pub struct RecvFuture<T>{
    once:bool,
    try_count : usize,
    #[pin]
    // inner: Option<T>,
    chan : Arc<Channel<T>>,

}
}

impl<T> RecvFuture<T> {
    pub fn new(once: bool, chan: Arc<Channel<T>>) -> RecvFuture<T> {
        RecvFuture {
            once,
            try_count: 0,
            chan,
        }
    }
}

// impl< T> Unpin for SendWait< T> {}
impl<T: Unpin> Future for RecvFuture<T> {
    type Output = ChannelResult<T, ()>;

    fn poll(self: Pin<&mut RecvFuture<T>>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        if this.chan.len() == 0 {
            if *this.once {
                //只是尝试发一次
                return Poll::Ready(empty_result());
            }
            if !this.chan.status() {
                return Ready(Err(ChannelError::CLOSED(())));
            }
            if !this.chan.recv_waker_buf().add(cx) {
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
        }
        let result = this.chan.try_recv_lock();
        let err = match result {
            None => {
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            Some(idx) => {
                let res = this.chan.unsafe_recv(idx);
                this.chan.unlock(idx);
                match res {
                    Ok(_) => {
                        this.chan.send_waker_buf().wake(1);
                        return Poll::Ready(res);
                    }
                    Err(err) => err,
                }
            }
        };
        match err {
            ChannelError::EMPTY => {
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            ChannelError::CLOSED(_) => return Poll::Ready(Err(err)),
            ChannelError::FULL(_) => {
                panic!("recv msg，but buf is full")
            }
        }
    }
}
