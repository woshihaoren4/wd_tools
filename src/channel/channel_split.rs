use crate::channel::{Channel, ChannelResult, RecvError, RecvFuture, SendError, SendFuture};
use std::sync::Arc;

#[derive(Debug)]
pub struct Sender<T> {
    chan: Arc<Channel<T>>,
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Self {
            chan: self.chan.clone(),
        }
    }
}

impl<T> From<Arc<Channel<T>>> for Sender<T> {
    fn from(chan: Arc<Channel<T>>) -> Self {
        Sender { chan }
    }
}

impl<T> Sender<T> {
    pub fn try_send(&self, data: T) -> ChannelResult<(), SendError<T>> {
        self.chan.try_send(data)
    }
    pub fn send(&self, value: T) -> SendFuture<T> {
        self.chan.send(value)
    }
    pub fn close(&self) {
        self.chan.close()
    }
    pub fn is_closed(&self) -> bool {
        self.chan.is_closed()
    }
}

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

impl<T> From<Arc<Channel<T>>> for Receiver<T> {
    fn from(chan: Arc<Channel<T>>) -> Self {
        Receiver { chan }
    }
}

impl<T> Receiver<T> {
    pub fn try_recv(&self) -> ChannelResult<T, RecvError> {
        self.chan.try_recv()
    }
    pub fn recv(&self) -> RecvFuture<T> {
        self.chan.recv()
    }
    pub fn close(&self) {
        self.chan.close()
    }
    pub fn is_closed(&self) -> bool {
        self.chan.is_closed()
    }
}

impl<T> Channel<T> {
    pub fn new(cap: usize) -> (Sender<T>, Receiver<T>) {
        let chan = Arc::new(Channel::with_cap(cap));
        (Sender::from(chan.clone()), Receiver::from(chan))
    }
}
