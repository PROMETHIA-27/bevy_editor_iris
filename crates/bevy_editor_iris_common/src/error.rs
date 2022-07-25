use quinn::{ConnectError, ConnectionError, ReadExactError, WriteError};
use rcgen::RcgenError;
use thiserror::Error;
use tokio::sync::mpsc::error::TryRecvError;

use crate::asynchronous::{MessageBox, MessageRx, MessageTx};

/// Error that a [transaction](crate::interface::Transaction) may use
#[derive(Debug, Error)]
pub enum TransactionError {
    /// Transaction channel closed
    #[error("transaction channel closed")]
    ChannelClosed,
}

/// Error that an [interface](crate::interface::Interface) may use
#[derive(Debug, Error)]
pub enum InterfaceError {
    /// Occurs when attempting to use an [`crate::interface::Interface`] which has been [poisoned](std::sync::RwLock).
    #[error("the interface has been poisoned")]
    Poison,
    /// [`send`](crate::interface::Interface::send) failed
    #[error(transparent)]
    Send(#[from] tokio::sync::mpsc::error::SendError<(MessageTx, MessageRx)>),
    /// [`try_recv`](crate::interface::Interface::try_recv) failed
    #[error(transparent)]
    TryRecv(#[from] TryRecvError),
}

/// A top-level error from the remote thread, indicating why it failed.
#[derive(Debug, Error)]
pub enum RemoteThreadError {
    /// A connection failed to be established.
    #[error(transparent)]
    ConnectError(#[from] ConnectError),
    /// A connection was lost.
    #[error(transparent)]
    ConnectionError(#[from] ConnectionError),
    /// A filesystem error occurred.
    #[error(transparent)]
    FsWriteError(#[from] std::io::Error),
    /// A failure occurred while processing an incoming connection.
    #[error(transparent)]
    ProcessConnectionError(#[from] ProcessConnectionError),
    /// A failure occurred while using Rcgen
    #[error(transparent)]
    RcgenError(#[from] RcgenError),
    /// A failure occurred while using rustls.
    #[error(transparent)]
    RustlsError(#[from] rustls::Error),
    /// A miscellaneous error.
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send>),
}

/// An error that occurs when processing an incoming connection.
#[derive(Debug, Error)]
pub enum ProcessConnectionError {
    /// An error occurred while processing a [`Message`] to send.
    #[error(transparent)]
    ProcessChannel(#[from] ProcessChannelError),
    /// An error occurred while processing an incoming stream.
    #[error(transparent)]
    ProcessStream(#[from] ProcessStreamError),
}

/// An error that occurs when processing an incoming stream.
#[derive(Debug, Error)]
pub enum ProcessStreamError {
    /// The stream of bi-streams unexpectedly closed.
    #[error("bi streams closed")]
    BiStreamsClosed,
    /// The connection unexpectedly closed.
    #[error(transparent)]
    Connection(#[from] ConnectionError),
    /// Failed to send a message to the local threads.
    #[error(transparent)]
    Send(#[from] tokio::sync::mpsc::error::SendError<(MessageTx, MessageRx)>),
    /// Failed to receive a message from the remote application.
    #[error(transparent)]
    Recv(#[from] RecvError),
}

/// An error that occurs while processing a message to send.
#[derive(Debug, Error)]
pub enum ProcessChannelError {
    /// The connection was unexpectedly lost.
    #[error(transparent)]
    ConnectionError(#[from] ConnectionError),
    /// The [opening channel](OpeningSender) between the remote and local threads has been closed
    #[error("the opening channel has been closed")]
    OpenChannelClosed,
    /// Failed to write to the remote stream.
    #[error(transparent)]
    StreamWriteError(#[from] WriteError),
}

/// An error that occurs when attempting to receive a message from the remote application
#[derive(Debug, Error)]
pub enum RecvError {
    /// Message deserialization failed.
    #[error("failed to deserialize message from client")]
    DeserializationFailed(#[from] MessageDeserError),
    /// The message header was invalid, indicating corrupt or malicious
    /// data is being sent.
    #[error("received malformed message header {:?}", .0)]
    InvalidData([u8; 4]),
    /// The stream unexpectedly closed before all data could be received.
    #[error(transparent)]
    ReadExact(#[from] ReadExactError),
    /// Failed to send a message to the local threads.
    #[error(transparent)]
    Send(#[from] tokio::sync::mpsc::error::SendError<MessageBox>),
}

/// An error that occurs when trying to send a message to the remote application
#[derive(Debug, Error)]
pub enum SendError {
    /// The [message channel](MessageTx) for this transaction was closed prematurely
    #[error("the message channel between this transaction and the local thread is closed")]
    ChannelClosed,
    /// An error occurred while serializing the message.
    #[error(transparent)]
    SerdeYamlError(#[from] serde_yaml::Error),
    /// The local thread sent a [signal](CloseTransaction) to close this transaction, and it was closed.
    /// This error should always be recovered from, as it indicates normal operations.
    #[error("the local thread closed this transaction")]
    TransactionClosed,
    /// Failed to write to the remote stream.
    #[error(transparent)]
    WriteError(#[from] WriteError),
}

/// An error that occurs while deserializing a [`Message`].
#[derive(Debug, Error)]
pub enum MessageDeserError {
    /// An error occurred during serialization
    #[error(transparent)]
    YamlError(#[from] serde_yaml::Error),
    /// The message type is not registered in the TypeRegistry
    #[error("the received message {} is not registered in the TypeRegistry", .0)]
    MessageNotRegistered(String),
    /// The message does not implement FromReflect or does not reflect the trait implementation
    #[error("the received message {} does not have an accessible FromReflect implementation; make sure to use #[reflect(MessageFromReflect)]", .0)]
    MessageNotFromReflect(String),
    /// The message failed to be converted using FromReflect
    #[error("the received message could not be converted to a concrete type: {:#?}", .0)]
    FromReflectFailed(String),
    /// The message does not implement [`Message`] or does not use #\[reflect(Message)]
    #[error("the received message {} does not have an accessible Message implementation; make sure to use #[reflect(Message)] or #[message]", .0)]
    MessageNotImpl(String),
}
