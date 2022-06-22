pub mod asynchronous;
mod interface;
pub mod message;
mod serde;
mod stream_pool;

pub use self::serde::*;
pub use interface::*;
pub use stream_pool::*;

pub use message::{
    AppRegisterMsgExt, DefaultMessages, Is, Message, MessageDistributor, MessageReceived,
    ReflectMessage, ReflectMessageFromReflect, RegisterMessage, SendMessage,
};

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct StreamId(pub(crate) usize);
