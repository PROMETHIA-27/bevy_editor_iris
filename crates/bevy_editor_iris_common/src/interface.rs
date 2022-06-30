use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TryRecvError;

use crate::asynchronous::{MessageBox, MessageRx, MessageTx, OpeningReceiver, OpeningSender};
use crate::error::{InterfaceError, TransactionError};
use crate::Message;

/// An interface to send and receive [messages](Message) to/from the remote application
pub struct Transaction {
    tx: MessageTx,
    rx: MessageRx,
}

/// A [cloneable](Clone) interface to send [messages](Message) to the remote application
#[derive(Clone)]
pub struct TransactionSender {
    tx: MessageTx,
}

/// An interface to receive [messages](Message) from the remote application
pub struct TransactionReceiver {
    rx: MessageRx,
}

impl Transaction {
    /// Split the transaction into a sender and receiver, allowing
    /// the sender to be cloned and the functionality to be separated
    #[inline]
    pub fn split(self) -> (TransactionSender, TransactionReceiver) {
        (
            TransactionSender { tx: self.tx },
            TransactionReceiver { rx: self.rx },
        )
    }

    /// Block until a [message](Message) is received
    #[inline]
    pub fn recv(&mut self) -> Option<MessageBox> {
        self.rx.blocking_recv()
    }

    /// Attempt to receive a [message](Message) or return
    /// a [TryRecvError] on failure
    #[inline]
    pub fn try_recv(&mut self) -> Result<MessageBox, TryRecvError> {
        self.rx.try_recv()
    }

    /// Send a message to the remote application through this transaction.
    /// Returns [TransactionError::ChannelClosed] if the transaction channel is closed.
    #[inline]
    pub fn send<M: Message>(&self, message: M) -> Result<(), TransactionError> {
        self.tx
            .send(Box::new(message))
            .map_err(|_| TransactionError::ChannelClosed)
    }
}

impl TransactionSender {
    /// Send a message to the remote application through this transaction.
    /// Returns [TransactionError::ChannelClosed] if the transaction channel is closed.
    #[inline]
    pub fn send<M: Message>(&self, message: M) -> Result<(), TransactionError> {
        self.tx
            .send(Box::new(message))
            .map_err(|_| TransactionError::ChannelClosed)
    }
}

impl TransactionReceiver {
    /// Block until a [message](Message) is received
    #[inline]
    pub fn recv(&mut self) -> Option<MessageBox> {
        self.rx.blocking_recv()
    }

    /// Attempt to receive a [message](Message) or return
    /// a [TryRecvError] on failure
    #[inline]
    pub fn try_recv(&mut self) -> Result<MessageBox, TryRecvError> {
        self.rx.try_recv()
    }
}

pub(crate) struct InternalInterface {
    pub(crate) open_tx: OpeningSender,
    pub(crate) open_rx: OpeningReceiver,
}

/// Represents the communication interface between the remote thread
/// and local threads. Can send and receive messages or close transactions.
pub struct Interface {
    pub(crate) inner: Arc<Mutex<InternalInterface>>,
}

impl Interface {
    /// Create a new interface from channels
    pub fn new(open_tx: OpeningSender, open_rx: OpeningReceiver) -> Self {
        Self {
            inner: Arc::new(Mutex::new(InternalInterface { open_tx, open_rx })),
        }
    }

    /// Attempts to retrieve the next [transaction](Transaction) stream. Fails if no transaction is ready yet
    /// or if the transaction channel was disconnected
    pub fn try_recv(&self) -> Result<Transaction, InterfaceError> {
        let mut lock = self.inner.lock().map_err(|_| InterfaceError::Poison)?;

        let (tx, rx) = lock.open_rx.try_recv()?;

        Ok(Transaction { tx, rx })
    }

    /// Attempts to open a new [transaction](Transaction) stream. Fails if the transaction channel was disconnected
    pub fn open_transaction(&self) -> Result<Transaction, InterfaceError> {
        let lock = self.inner.lock().map_err(|_| InterfaceError::Poison)?;

        let (tx, remote_rx) = mpsc::unbounded_channel();
        let (remote_tx, rx) = mpsc::unbounded_channel();

        lock.open_tx.send((remote_tx, remote_rx))?;

        Ok(Transaction { tx, rx })
    }
}

impl Clone for Interface {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}
