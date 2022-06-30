use thiserror::Error;
use tokio::sync::mpsc::error::{SendError, TryRecvError};

use crate::asynchronous::{MessageRx, MessageTx};

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
    Send(#[from] SendError<(MessageTx, MessageRx)>),
    /// [`try_recv`](crate::interface::Interface::try_recv) failed
    #[error(transparent)]
    TryRecv(#[from] TryRecvError),
}
