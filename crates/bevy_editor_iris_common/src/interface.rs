use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use thiserror::Error;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::asynchronous::{OpeningReceiver, OpeningSender};
use crate::Message;

use super::message::messages::CloseTransaction;

/// An ID of a transaction's stream. Useful for sending messages to
/// or receiving messages from a particular transaction.
///
/// Received when sending a message via an [`Interface`] or from a [`StreamCounter`]
// #[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
// pub struct StreamId(pub(crate) usize);

/// The atomic counter for streams. This is used to generate stream IDs
/// without overlapping.
// #[derive(Clone)]
// pub struct StreamCounter(pub(crate) Arc<AtomicUsize>);

// impl Default for StreamCounter {
//     fn default() -> Self {
//         Self(Arc::new(AtomicUsize::new(0)))
//     }
// }

// impl StreamCounter {
//     /// Get the next available [`StreamId`].
//     pub fn next(&self) -> StreamId {
//         // TODO: Not certain whether or not strict ordering is required
//         let id = self.0.fetch_add(1, Ordering::SeqCst);
//         StreamId(id)
//     }
// }

pub(crate) struct InternalInterface {
    pub(crate) open_tx: OpeningSender,
    pub(crate) open_rx: OpeningReceiver,
}

/// Represents the communication interface between the remote thread
/// and local threads. Can send and receive messages or close transactions.
pub struct Interface {
    pub(crate) inner: Arc<Mutex<InternalInterface>>,
    // pub(crate) stream_counter: StreamCounter,
}

impl Interface {
    /// Create a new interface by constructing it from channels and a StreamCounter.
    /// The interface should only interact with StreamIds produced from this StreamCounter.
    pub fn new(
        open_tx: OpeningSender,
        open_rx: OpeningReceiver,
        // stream_counter: StreamCounter,
    ) -> Self {
        Self {
            inner: Arc::new(Mutex::new(InternalInterface { open_tx, open_rx })),
            // stream_counter,
        }
    }

    // TODO: Throw out WaitHandle and switch to recv()/try_recv()
    // / Grab a [`WaitHandle`] for the next message this interface receives on any stream.
    // / This method will not block, but the method [`WaitHandle::wait()`] allows blocking
    // / until a message is received.
    // pub fn recv(&self) -> WaitHandle<(StreamId, Box<dyn Message>)> {
    //     WaitHandle {
    //         interface: self.clone(),
    //         poll_fn: Box::new(|interface| match interface.inner.lock() {
    //             Ok(inner) => match inner.incoming.try_recv() {
    //                 Ok(msg) => Some(Ok(msg)),
    //                 Err(TryRecvError::Disconnected) => Some(Err(InterfaceError::Disconnected)),
    //                 Err(TryRecvError::Empty) => None,
    //             },
    //             Err(_) => Some(Err(InterfaceError::Poison)),
    //         }),
    //     }
    // }

    // / Collect all messages this interface currently has available. Will return immediately.
    // pub fn recv_all(&self) -> Result<Vec<(StreamId, Box<dyn Message>)>, InterfaceError> {
    //     let inner = self.inner.lock().map_err(|_| InterfaceError::Poison)?;

    //     Ok(inner.incoming.try_iter().collect())
    // }

    // / Send a message to the given transaction.
    // / If no [`StreamId`] is provided, begin a new transaction.
    // /
    // / Will always return the [`StreamId`] of the transaction the message
    // / was sent to.
    // pub fn send(
    //     &self,
    //     id: Option<StreamId>,
    //     msg: Box<dyn Message>,
    // ) -> Result<StreamId, InterfaceError> {
    //     let id = match id {
    //         Some(id) => id,
    //         None => self.stream_counter.next(),
    //     };

    //     match self.inner.lock() {
    //         Ok(inner) => match inner.outgoing.send(msg) {
    //             Ok(()) => Ok(id),
    //             Err(_) => Err(InterfaceError::Disconnected),
    //         },
    //         Err(_) => Err(InterfaceError::Poison),
    //     }
    // }

    // / Send multiple messages at once, immediately returning [`InterfaceError::Poison`] if
    // / the interface is poisoned, or returning [`InterfaceError::Disconnected`] as soon
    // / as a message fails to send. The remaining iterator is returned on a failure.
    // pub fn send_all<I: Iterator<Item = Box<dyn Message>>>(
    //     &self,
    //     mut messages: I,
    // ) -> Result<(), (InterfaceError, I)> {
    //     let inner = match self.inner.lock() {
    //         Ok(inner) => inner,
    //         Err(_) => return Err((InterfaceError::Poison, messages)),
    //     };

    //     while let Some(msg) = messages.next() {
    //         if let Err(_) = inner.outgoing.send(msg) {
    //             return Err((InterfaceError::Disconnected, messages));
    //         }
    //     }

    //     Ok(())
    // }

    // / Close the given transaction. This sends the [`CloseTransaction`] message and
    // / afterwards, the [`StreamId`] is no longer usable.
    // pub fn close(&self, id: StreamId) -> Result<(), InterfaceError> {
    //     match self.inner.lock() {
    //         Ok(inner) => match inner.outgoing.send(Box::new(CloseTransaction)) {
    //             Ok(()) => Ok(()),
    //             Err(_) => Err(InterfaceError::Disconnected),
    //         },
    //         Err(_) => Err(InterfaceError::Poison),
    //     }
    // }
}

impl Clone for Interface {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            // stream_counter: self.stream_counter.clone(),
        }
    }
}

/// An error that occurs when attempting to send or receive messages through an [`Interface`].
#[derive(Debug, Error)]
pub enum InterfaceError {
    /// Occurs when attempting to send or receive messages from a stream which no longer
    /// has a valid connection.
    #[error("stream disconnected")]
    Disconnected,
    /// Occurs when attempting to use an [`Interface`] which has been [poisoned](std::sync::RwLock).
    #[error("the interface has been poisoned")]
    Poison,
}

/// The result of polling a [`WaitHandle`].
///
/// Possible values:
/// - `None`: No value ready yet.
/// - `Some(Ok(Output))`: Value ready.
/// - `Some(Err(InterfaceError))`: An error occurred retrieving the value.
pub type WaitHandleResult<Output> = Option<Result<Output, InterfaceError>>;

/// Represents a value that will be ready at some point in the future, much like an async
/// [`std::future::Future`]. Can be [polled](WaitHandle::poll()) or [waited on](WaitHandle::wait()).
pub struct WaitHandle<Output> {
    interface: Interface,
    // It would be possible to use non-dynamic types here to improve performance but I couldn't
    // figure out how to do so without mangling all the types and it being really unwieldy
    poll_fn: Box<dyn FnMut(&mut Interface) -> WaitHandleResult<Output>>,
}

impl<Output: 'static> WaitHandle<Output> {
    /// Poll to retrieve the value (or [`InterfaceError`]) if it's ready, or [`None`] if not.
    #[inline]
    pub fn poll(&mut self) -> WaitHandleResult<Output> {
        (self.poll_fn)(&mut self.interface)
    }

    /// Block on this [`WaitHandle`] until the value (or [`InterfaceError`]) is ready.
    pub fn wait(&mut self) -> Result<Output, InterfaceError> {
        loop {
            if let Some(result) = self.poll() {
                break result;
            }
        }
    }

    /// Map the return value of this [`WaitHandle`] with `f`.
    pub fn map<New, F: 'static + FnMut(Output) -> New>(self, mut f: F) -> WaitHandle<New> {
        let WaitHandle {
            interface,
            mut poll_fn,
        } = self;

        WaitHandle {
            interface,
            poll_fn: Box::new(move |interface| match poll_fn(interface) {
                Some(Ok(output)) => Some(Ok(f(output))),
                Some(Err(err)) => Some(Err(err)),
                None => None,
            }),
        }
    }
}
