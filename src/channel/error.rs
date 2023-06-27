use std::fmt::{Display, Formatter};

#[derive(Debug,PartialEq)]
pub enum ChannelError<T>{
    EMPTY,
    CLOSED(T),
    FULL(T),
}

impl<T> Display for ChannelError<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelError::EMPTY => write!(f, "Channel is EMPTY"),
            ChannelError::CLOSED(_) => write!(f, "Channel is CLOSED"),
            ChannelError::FULL(_) => write!(f, "Channel is FULL")
        }
    }
}

pub type ChannelResult<T,Val> = Result<T,ChannelError<Val>>;


pub(crate) fn empty_result<T>()->ChannelResult<T,()>{
    Err(ChannelError::EMPTY)
}
pub(crate) fn close_result<T>(val:T)->ChannelResult<(),T>{
    Err(ChannelError::CLOSED(val))
}
pub(crate) fn full_result<T>(val:T)->ChannelResult<(),T>{
    Err(ChannelError::FULL(val))
}
pub(crate) fn send_success_result<Val>()->ChannelResult<(),Val>{
    Ok(())
}
pub(crate) fn recv_success_result<T>(t:T)->ChannelResult<T,()>{
    Ok(t)
}
