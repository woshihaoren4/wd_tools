use std::fmt::{Debug, Display, Formatter};

pub trait ChannelError: Display {
    fn into_err<T>(self) -> ChannelResult<T, Self>
    where
        Self: Sized,
    {
        ChannelResult::Err(self)
    }
}

#[derive(PartialEq)]
pub enum SendError<T> {
    CLOSED(T),
    FULL(T),
    UNKNOWN(T, String),
}
impl<T> Display for SendError<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SendError::CLOSED(_) => write!(f, "ChannelClose"),
            SendError::FULL(_) => write!(f, "ChannelFull"),
            SendError::UNKNOWN(_, e) => write!(f, "ChannelUnknown error:{e}"),
        }
    }
}

impl<T> ChannelError for SendError<T> {}
impl<T> Debug for SendError<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}
impl<T> std::error::Error for SendError<T> {}

#[derive(PartialEq)]
pub enum RecvError {
    CLOSED,
    EMPTY,
    UNKNOWN(String),
}
impl Display for RecvError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RecvError::CLOSED => write!(f, "ChannelClose"),
            RecvError::EMPTY => write!(f, "ChannelEmpty"),
            RecvError::UNKNOWN(e) => write!(f, "ChannelUnknown error:{e}"),
        }
    }
}

impl ChannelError for RecvError {}
impl Debug for RecvError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}
impl std::error::Error for RecvError {}

pub type ChannelResult<T, E> = Result<T, E>;

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
