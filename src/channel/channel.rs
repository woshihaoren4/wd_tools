use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::task::{Context, Waker};
use crate::channel::*;

#[derive(Debug)]
pub struct Channel<T>{
    status : Arc<AtomicBool>,
    cap : usize,
    send_idx : AtomicUsize,
    recv_idx : AtomicUsize,
    // len : AtomicUsize,
    // lock: AtomicBool,
    buf : Vec<Slot<T>>,
    send_buf: SlotWaker,
    recv_buf: SlotWaker
}

unsafe impl<T> Send for Channel<T>{}
unsafe impl<T> Sync for Channel<T>{}

#[derive(Debug)]
struct Slot<T>{
    lock : AtomicBool,
    some : AtomicBool,
    value: Option<T>
}

impl<T> Default for Slot<T> {
    fn default() -> Self {
        Slot{lock:AtomicBool::default(),some:AtomicBool::default(), value:None}
    }
}
impl<T> Slot<T> {
    fn is_none(&self)->bool{
        !self.some.load(Ordering::Relaxed)
    }
}
#[derive(Debug)]
pub struct SlotWaker{
    status : Arc<AtomicBool>,
    lock : AtomicBool,
    len : AtomicUsize,
    buf: VecDeque<Waker>,
}



impl SlotWaker {
    fn new(status : Arc<AtomicBool>) -> Self {
        Self{status,lock:AtomicBool::default(),len:AtomicUsize::default(),buf:VecDeque::new()}
    }
}

impl SlotWaker {
    pub(crate) fn len(&self)->usize{
        self.len.load(Ordering::Relaxed)
    }
    fn lock(&self)->bool{
        let res = self.lock.compare_exchange_weak(false, true,Ordering::SeqCst,Ordering::Relaxed);
        return res.is_ok()
    }

    fn unlock(&self){
        self.lock.store(false,Ordering::Relaxed);
    }
    pub(crate) fn add(&self,cx:&Context)->bool{
        if !self.lock() {
            return false
        }
        if !self.status.load(Ordering::Relaxed) {
            return false
        }
        unsafe {
            let buf = & mut*(&self.buf as *const VecDeque<Waker> as *mut VecDeque<Waker>);
            buf.push_back(cx.waker().clone());
        }
        self.len.fetch_add(1,Ordering::Relaxed);
        self.unlock();
        true
    }
    pub(crate) fn wake(&self,len:usize)->bool{
        if !self.lock() {
            return false
        }
        unsafe {
            let buf = & mut*(&self.buf as *const VecDeque<Waker> as *mut VecDeque<Waker>);
            for _ in 0..len{
                match buf.pop_front() {
                    None => break,
                    Some(w) => {
                        w.wake_by_ref();
                        self.len.fetch_sub(1,Ordering::Relaxed);
                    }
                }
            }
        }
        self.unlock();
        true
    }
}

impl<T> Channel<T>
where T:Unpin
{
    pub fn with_capacity(cap:usize)->Channel<T>{
        let mut buf = Vec::with_capacity(cap);
        for _ in 0..cap{
            buf.push(Slot::default())
        }
        let status = Arc::new(AtomicBool::new(true));
        let send_idx = AtomicUsize::default();
        let recv_idx = AtomicUsize::default();
        let send_buf = SlotWaker::new(status.clone());
        let recv_buf = SlotWaker::new(status.clone());
        Channel {
            status,
            cap,
            buf,
            send_idx,
            recv_idx,
            send_buf,
            recv_buf
        }
    }
    pub(crate) fn status(&self)->bool{
        self.status.load(Ordering::Relaxed)
    }
    pub(crate) fn close(&self){
        self.status.store(false,Ordering::Relaxed);
    }
    pub(crate) fn len(&self) ->usize {
        let si = self.send_idx.load(Ordering::Relaxed);
        let ri = self.recv_idx.load(Ordering::Relaxed);
        if si < ri {
            return 0
        }
        return si - ri;
    }
    pub(crate) fn cap(&self)->usize{
        self.cap
    }
    pub(crate) fn send_waker_buf(&self)->&SlotWaker{
        &self.send_buf
    }
    pub(crate) fn recv_waker_buf(&self)->&SlotWaker{
        &self.recv_buf
    }
    pub(crate) fn try_send_lock(&self)->Option<usize>{
        let si = self.send_idx.load(Ordering::Relaxed);
        let ri = self.recv_idx.load(Ordering::Relaxed);
        if si >= ri + self.cap {
            return None
        }

        for i in 0..self.cap{
            let index = (si + i) % self.cap;
            if !self.buf[index].is_none() {
                continue
            }
            let res = (&self.buf)[index].lock.compare_exchange_weak(false, true,Ordering::SeqCst,Ordering::Relaxed);
            if res.is_ok() {
                if self.buf[index].value.is_some(){
                    self.unlock(index);
                }else {
                    return Some(index)
                }
            }
            if self.len() >= self.cap {
                return None
            }
        }
        return None
    }
    pub(crate) fn try_recv_lock(&self)->Option<usize>{
        let si = self.send_idx.load(Ordering::Relaxed);
        let ri = self.recv_idx.load(Ordering::Relaxed);
        if ri >= si {
            return None
        }
        for i in 0..self.cap{
            let index = (ri + i) % self.cap;
            if self.buf[index].is_none() {
                continue
            }
            let res = self.buf[index].lock.compare_exchange_weak(false, true,Ordering::SeqCst,Ordering::Relaxed);
            if res.is_ok() {
                if self.buf[index].value.is_none(){
                    self.unlock(index);
                }else{
                    return Some(index)
                }
            }
            if self.len() == 0 {
                return None
            }
        }
        return None
    }
    pub(crate) fn unlock(&self,idx:usize){
        self.buf[idx].lock.store(false,Ordering::Relaxed);
    }
    pub(crate) fn unsafe_send(&self, t:T, idx : usize) ->ChannelResult<(),T>{
        if self.buf[idx].value.is_some() {
            return full_result(t)
        }
        unsafe {
            let buf = &mut*(&self.buf as *const Vec<Slot<T>> as *mut Vec<Slot<T>>);
            buf[idx].value.replace(t);
            buf[idx].some.store(true,Ordering::Relaxed);
        }
        self.send_idx.fetch_add(1,Ordering::Relaxed);
        send_success_result()
    }
    pub(crate) fn unsafe_recv(&self, idx : usize) ->ChannelResult<T,()>{
        if self.buf[idx].value.is_none() {
            return empty_result()
        }
        unsafe {
            let buf = &mut*(&self.buf as *const Vec<Slot<T>> as *mut Vec<Slot<T>>);
            let opt = buf[idx].value.take();
            buf[idx].some.store(false,Ordering::Relaxed);
            self.recv_idx.fetch_add(1,Ordering::Relaxed);
            recv_success_result(opt.unwrap())
        }

    }
}


