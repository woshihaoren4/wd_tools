use std::fmt::{Display, Formatter};

pub trait ChannelError:Display {
    fn into_err<T>(self)->ChannelResult<T,Self>{
        ChannelResult::Err(self)
    }
}

#[derive(Debug, PartialEq)]
pub enum SendError<T> {
    CLOSED(T),
    FULL(T),
    UNKNOWN(T,String),
}
impl<T> Display for SendError<T>   {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SendError::CLOSED(_) =>  write!(f, "Channel is EMPTY"),
            SendError::FULL(_) =>  write!(f, "Channel is full"),
            SendError::UNKNOWN(_,e) =>  write!(f, "ChannelUnknown error:{e}"),
        }
    }
}

impl<T> ChannelError for SendError<T> {}

pub type ChannelResult<T,E:ChannelError> = Result<T,E>;

// impl<T> Display for ChannelError<T> {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         match self {
//             ChannelError::EMPTY => write!(f, "Channel is EMPTY"),
//             ChannelError::CLOSED(_) => write!(f, "Channel is CLOSED"),
//             ChannelError::FULL(_) => write!(f, "Channel is FULL"),
//             ChannelError::UNKNOWN(info) => write!(f, "ChannelUnknownError:{info}"),
//         }
//     }
// }

// pub type ChannelResult<T, Val> = Result<T, ChannelError<Val>>;
// 
// pub(crate) fn empty_result<T>() -> ChannelResult<T, ()> {
//     Err(ChannelError::EMPTY)
// }
// pub(crate) fn close_result<T>(val: T) -> ChannelResult<(), T> {
//     Err(ChannelError::CLOSED(val))
// }
// pub(crate) fn full_result<T>(val: T) -> ChannelResult<(), T> {
//     Err(ChannelError::FULL(val))
// }
// pub(crate) fn send_success_result<Val>() -> ChannelResult<(), Val> {
//     Ok(())
// }
// pub(crate) fn recv_success_result<T>(t: T) -> ChannelResult<T, ()> {
//     Ok(t)
// }

// impl<T> From<ChannelError<T>> for ChannelResult<(),T>{
//     fn from(value: ChannelError<T>) -> Self {
//         Self::Err(value)
//     }
// }