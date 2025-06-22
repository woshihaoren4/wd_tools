use crate::channel::*;
use std::collections::VecDeque;
use std::future::Future;
use std::ops::DerefMut;
use std::pin::{pin, Pin};
use std::sync::{Arc, LockResult, Mutex};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::task::{Context, Poll, Waker};
use pin_project_lite::pin_project;

#[derive(Debug)]
pub struct Channel<T> {
    status: Arc<AtomicBool>,
    cap: usize,
    wait_deque: Arc<Mutex<WaitDeque<T>>>
}

#[derive(Debug)]
pub struct WaitDeque<T>{
    deque: VecDeque<T>,
    sender_waker: VecDeque<Waker>,
    receiver_waker: VecDeque<Waker>,
}
impl<T> Clone for Channel<T>{
    fn clone(&self) -> Self {
        Self{
            status:self.status.clone(),
            cap:self.cap,
            wait_deque:self.wait_deque.clone(),
        }
    }
}

unsafe impl<T> Send for Channel<T> {}
unsafe impl<T> Sync for Channel<T> {}

pin_project! {
    pub struct SenderFuture<T>{
    data: Option<T>,
    chan: Channel<T>,
    }
}
impl<T> Future for SenderFuture<T> {
    type Output = ChannelResult<(),T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let data = this.data.deref_mut();
        let cap = this.chan.cap;
        let res = this.chan._try_send(data,|c,d|{
            if c.deque.len() >= cap {
                c.sender_waker.push_back(cx.waker().clone());
                Ok(Poll::Pending)
            } else {
                let data =d.take().unwrap();
                c.deque.push_back(data);
                if let Some(recv) = c.receiver_waker.pop_front() {
                    recv.wake();
                }
                Ok(Poll::Ready(ChannelResult::Ok(())))
            }
        });
        res.unwrap_or_else(|e| Poll::Ready(ChannelResult::Err(e)))
    }
}

impl<T> Channel<T> {
    pub fn new(mut cap: usize) -> Channel<T> {
        if cap <= 0 {
            cap = 1;
        }
        let deque = VecDeque::with_capacity(cap);
        Channel{
            status: Arc::new(AtomicBool::new(true)),
            cap,
            wait_deque: Arc::new(Mutex::new(WaitDeque{
                deque,
                sender_waker: VecDeque::new(),
                receiver_waker: VecDeque::new(),
            }))
        }
    }
    pub fn get_status(&self) -> bool {
        self.status.load(Ordering::Relaxed)
    }
    pub(crate) fn _try_send<Out>(&self, data:&mut Option<T>,send_handle:impl FnOnce(&mut WaitDeque<T>,&mut Option<T>)->ChannelResult<Out, SendError<T>>) -> ChannelResult<Out, SendError<T>> {
        if !self.get_status() {
            let data = data.take().unwrap();
            return SendError::CLOSED(data).into_err()
        }
        let mut lock = match self.wait_deque.lock() {
            Ok(o) => o,
            Err(e) => {
                let data = data.take().unwrap();
                return SendError::UNKNOWN(data,e.to_string()).into_err()
            }
        };
        send_handle(lock.deref_mut(),data)
    }
    pub fn send(&self, value: T) -> SenderFuture<T> {
        SenderFuture {
            data: Some(value),
            chan: self.clone(),
        }
    }
}