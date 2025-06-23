use crate::channel::*;
use std::collections::VecDeque;
use std::future::Future;
use std::ops::DerefMut;
use std::pin::{Pin};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
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
    pub struct SendFuture<T>{
    data: Option<T>,
    chan: Channel<T>,
    }
}
impl<T> Future for SendFuture<T> {
    type Output = ChannelResult<(),SendError<T>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
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
        match res {
            Ok(o) => o,
            Err(e)=>{
                Poll::Ready(ChannelResult::Err(e))
            }
        }
    }
}

pin_project! {
    pub struct RecvFuture<T>{
        chan: Channel<T>,
    }
}

impl<T> Future for RecvFuture<T> {
    type Output = ChannelResult<T, RecvError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let res = this.chan._try_recv(|c|{
            c.receiver_waker.push_back(cx.waker().clone());
        });
        match res {
            Ok(t) => Poll::Ready(ChannelResult::Ok(t)),
            Err(e) => {
                match e {
                    RecvError::EMPTY => Poll::Pending,
                    _ => Poll::Ready(e.into_err()),
                }
            }
        }
    }
}

impl<T> Channel<T> {
    pub fn with_cap(mut cap: usize) -> Channel<T> {
        if cap <= 0 {
            cap = 1;
        }
        let deque = VecDeque::with_capacity(cap);
        Channel {
            status: Arc::new(AtomicBool::new(true)),
            cap,
            wait_deque: Arc::new(Mutex::new(WaitDeque {
                deque,
                sender_waker: VecDeque::new(),
                receiver_waker: VecDeque::new(),
            }))
        }
    }
    pub fn get_status(&self) -> bool {
        self.status.load(Ordering::Relaxed)
    }
    pub(crate) fn _try_send<Out>(&self, data: &mut Option<T>, send_handle: impl FnOnce(&mut WaitDeque<T>, &mut Option<T>) -> ChannelResult<Out, SendError<T>>) -> ChannelResult<Out, SendError<T>> {
        if !self.get_status() {
            let data = data.take().unwrap();
            return SendError::CLOSED(data).into_err()
        }
        let mut lock = match self.wait_deque.lock() {
            Ok(o) => o,
            Err(e) => {
                let data = data.take().unwrap();
                return SendError::UNKNOWN(data, e.to_string()).into_err()
            }
        };
        send_handle(lock.deref_mut(), data)
    }
    pub fn try_send(&self, data: T) -> ChannelResult<(), SendError<T>> {
        let mut data = Some(data);
        let cap = self.cap;
        self._try_send(&mut data, |c, d| {
            let data = d.take().unwrap();
            if c.deque.len() >= cap {
                SendError::FULL(data).into_err()
            } else {
                c.deque.push_back(data);
                if let Some(recv) = c.receiver_waker.pop_front() {
                    recv.wake();
                }
                Ok(())
            }
        })
    }
    pub fn send(&self, value: T) -> SendFuture<T> {
        SendFuture {
            data: Some(value),
            chan: self.clone(),
        }
    }
    pub(crate) fn _try_recv(&self, empty: impl FnOnce(&mut WaitDeque<T>)) -> ChannelResult<T, RecvError> {
        let mut lock = match self.wait_deque.lock() {
            Ok(o) => o,
            Err(e) => {
                return RecvError::UNKNOWN(e.to_string()).into_err()
            }
        };
        if let Some(s) = lock.deque.pop_front() {
            if let Some(w) = lock.sender_waker.pop_front() {
                w.wake();
            }
            return Ok(s)
        } else {
            if !self.get_status() {
                return RecvError::CLOSED.into_err()
            }
            empty(lock.deref_mut());
        }
        RecvError::EMPTY.into_err()
    }
    pub fn try_recv(&self)->ChannelResult<T, RecvError>{
        self._try_recv(|_e|{})
    }
    pub fn recv(&self) -> RecvFuture<T> {
        RecvFuture{
            chan: self.clone(),
        }
    }
    pub fn close(&self) {
        self.status.store(false, Ordering::Relaxed);
        let mut lock = self.wait_deque.lock().unwrap();
        for i in lock.receiver_waker.drain(..){
            i.wake();
        }
        for i in lock.sender_waker.drain(..){
            i.wake();
        }
    }
    pub fn is_closed(&self) -> bool {
        !self.status.load(Ordering::Relaxed)
    }
}