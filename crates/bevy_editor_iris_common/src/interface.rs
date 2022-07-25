use std::any::TypeId;
use std::sync::{Arc, Mutex};

use bevy::ecs::system::Command;
use bevy::prelude::Commands;
use bevy::reflect::{FromReflect, Reflect};
use bevy::utils::HashMap;
use bevy_editor_iris_derive::{message, Message};
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TryRecvError;

use crate::asynchronous::{MessageBox, MessageRx, MessageTx, OpeningReceiver, OpeningSender};
use crate::error::{InterfaceError, TransactionError};
use crate::message::{Message, ReflectMessage, ReflectMessageFromReflect};

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
    /// Returns `true` if the sender has been closed or the receiver has been dropped.
    #[inline]
    pub fn sender_is_closed(&self) -> bool {
        self.tx.is_closed()
    }

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
    /// Returns [`TransactionError::ChannelClosed`] if the transaction channel is closed.
    #[inline]
    pub fn send<M: Message>(&self, message: M) -> Result<(), TransactionError> {
        self.tx
            .send(Box::new(message))
            .map_err(|_| TransactionError::ChannelClosed)
    }

    /// Get an iterator over incoming messages. Stops when an empty message is encountered
    /// or the transaction stream is closed.
    pub fn iter(&mut self) -> TransactionIterator {
        TransactionIterator { rx: &mut self.rx }
    }
}

/// An iterator over the incoming messages of a transaction
pub struct TransactionIterator<'rx> {
    rx: &'rx mut MessageRx,
}

impl<'rx> Iterator for TransactionIterator<'rx> {
    type Item = MessageBox;

    fn next(&mut self) -> Option<Self::Item> {
        if let Ok(msg) = self.rx.try_recv() {
            Some(msg)
        } else {
            None
        }
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

type MessageCallback = Box<dyn FnMut(MessageBox, Commands)>;

pub(crate) struct InternalInterface {
    pub(crate) open_tx: OpeningSender,
    pub(crate) open_rx: OpeningReceiver,
    pub(crate) callbacks: HashMap<TypeId, MessageCallback>,
    pub(crate) transactions: Vec<Transaction>,
}

// TODO: Should the interface be non-send instead of mutexed?
// TODO: Investigate systemparam for sender
/// Represents the communication interface between the remote thread
/// and local threads.
pub struct Interface {
    pub(crate) inner: Mutex<InternalInterface>,
}

impl Interface {
    /// Create a new interface from channels
    pub fn new(open_tx: OpeningSender, open_rx: OpeningReceiver) -> Self {
        Self {
            inner: Arc::new(Mutex::new(InternalInterface {
                open_tx,
                open_rx,
                callbacks: Default::default(),
                transactions: vec![],
            })),
        }
    }

    /// Attempts to retrieve the next [transaction](Transaction) stream. Fails if no transaction is ready yet.
    /// or if the transaction channel was disconnected
    pub(crate) fn try_recv(&mut self) -> Result<Transaction, InterfaceError> {
        let mut lock = self.inner.get_mut().map_err(|_| InterfaceError::Poison)?;

        let (tx, rx) = lock.open_rx.try_recv()?;

        Ok(Transaction { tx, rx })
    }

    /// Attempts to open a new [transaction](Transaction) stream. Fails if the transaction channel was disconnected.
    pub(crate) fn open_transaction(&mut self) -> Result<Transaction, InterfaceError> {
        let lock = self.inner.get_mut().map_err(|_| InterfaceError::Poison)?;

        let (tx, remote_rx) = mpsc::unbounded_channel();
        let (remote_tx, rx) = mpsc::unbounded_channel();

        lock.open_tx.send((remote_tx, remote_rx))?;

        Ok(Transaction { tx, rx })
    }

    /// Register a callback for a particular message type. This callback will be called at some point after receiving a message of that type.
    pub fn register_callback<M: Message>(&mut self, callback: impl FnMut(M, Commands)) {
        let callback = |msg: MessageBox, c| (callback)(msg.downcast().unwrap(), c);

        let lock = self.inner.get_mut().map_err(|_| InterfaceError::Poison)?;

        let old = lock.callbacks.insert(TypeId::of::<M>(), callback);
        // TODO: Might be a better way to handle this.
        assert!(old.is_none());
    }
}

impl Clone for Interface {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

/// A special message built into the editor.
/// When sent, closes a transaction on the local side.
#[message]
pub struct CloseTransaction;
