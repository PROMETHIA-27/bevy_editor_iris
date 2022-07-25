//! The [transaction registry](TransactionRegistry) is how incoming streams from the
//! remote application can be acquired. Every stream will have a first message;
//! a channel can be registered to receive a stream based on the [TypeId] of its first message.
//!
//! Alternatively, by running a system [before the TransactionRegistry is updated](RunTransactionRegistry),
//! streams can be acquired before they are distributed according to the transaction registry.
//! This may be useful if finer granularity is needed than the type of the first message.

use std::any::TypeId;
use std::sync::mpsc::Sender;

use bevy::prelude::{SystemLabel, World};
use bevy::utils::HashMap;

use crate::asynchronous::{MessageBox, MessageRx, MessageTx};
use crate::interface::Interface;
use crate::message::Message;

/// A registry of channels to send incoming streams to, based on
/// the [TypeId] of their first [message](Message).
#[derive(Default)]
pub struct TransactionRegistry {
    map: HashMap<TypeId, Sender<(MessageTx, MessageRx, MessageBox)>>,
    pool: HashMap<TypeId, Vec<(MessageTx, MessageRx, MessageBox)>>,
}

impl TransactionRegistry {
    /// Register a sender to receive new streams based on their
    /// first [message](Message). Returns the old sender if any was registered
    /// for this message type.
    ///
    /// If any matching streams are pooled, they will be sent
    /// immediately.
    ///
    /// If an old sender is replaced, manual synchronization may be
    /// needed to allow the previous consumer to continue functioning.
    pub fn register<M: Message>(
        &mut self,
        sender: Sender<(MessageTx, MessageRx, MessageBox)>,
    ) -> Option<Sender<(MessageTx, MessageRx, MessageBox)>> {
        match self.pool.remove(&TypeId::of::<M>()) {
            Some(pool) => {
                for stream in pool {
                    _ = sender.send(stream);
                }
            }
            None => (),
        }

        self.map.insert(TypeId::of::<M>(), sender)
    }

    /// Get the pool corresponding to streams with a first message of type M.
    pub fn pool<M: Message>(&mut self) -> Option<&mut Vec<(MessageTx, MessageRx, MessageBox)>> {
        self.pool.get_mut(&TypeId::of::<M>())
    }
}

/// The label for the system that updates the [transaction registry](TransactionRegistry)
#[derive(Clone, Debug, Eq, Hash, PartialEq, SystemLabel)]
pub struct RunTransactionRegistry;

pub(crate) fn update_transaction_registry(world: &mut World) {
    let mut registry: TransactionRegistry = world.remove_non_send_resource().unwrap();
    let interface: Interface = world.remove_resource().unwrap();

    let mut lock = match interface.inner.lock() {
        Ok(i) => i,
        Err(_) => return,
    };

    while let Ok((tx, mut rx)) = lock.open_rx.try_recv() {
        // According to quinn, streams will not be picked up by the recipient until they're used.
        // This should mean that this will never block for a significant amount of time.
        // TODO: Verify this assumption
        let first_msg = match rx.blocking_recv() {
            Some(msg) => msg,
            None => continue,
        };
        let id = first_msg.as_any().type_id();

        match registry.map.get(&id) {
            Some(entry) => _ = entry.send((tx, rx, first_msg)),
            None => match registry.pool.get_mut(&id) {
                Some(pool) => pool.push((tx, rx, first_msg)),
                None => {
                    registry.pool.insert(id, vec![(tx, rx, first_msg)]);
                }
            },
        }
    }

    drop(lock);

    world.insert_non_send_resource(registry);
    world.insert_resource(interface);
}
