use crate::common::*;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc::{Receiver, Sender, TryRecvError},
};
use std::sync::{Arc, Mutex};
use thiserror::Error;

use super::message::CloseTransaction;

pub(crate) struct InternalInterface {
    pub(crate) incoming: Receiver<(StreamId, Box<dyn Message>)>,
    pub(crate) outgoing: Sender<(StreamId, Box<dyn Message>)>,
    pub(crate) stream_counter: Arc<AtomicUsize>,
}

pub struct Interface {
    pub(crate) inner: Arc<Mutex<InternalInterface>>,
}

impl Interface {
    pub fn new(
        incoming: Receiver<(StreamId, Box<dyn Message>)>,
        outgoing: Sender<(StreamId, Box<dyn Message>)>,
        stream_counter: Arc<AtomicUsize>,
    ) -> Self {
        Self {
            inner: Arc::new(Mutex::new(InternalInterface {
                outgoing,
                incoming,
                stream_counter,
            })),
        }
    }

    pub fn recv(&self) -> WaitHandle<(StreamId, Box<dyn Message>)> {
        WaitHandle {
            interface: self.clone(),
            poll_fn: Box::new(|interface| match interface.inner.lock() {
                Ok(inner) => match inner.incoming.try_recv() {
                    Ok(msg) => Some(Ok(msg)),
                    Err(TryRecvError::Disconnected) => Some(Err(InterfaceError::Disconnected)),
                    Err(TryRecvError::Empty) => None,
                },
                Err(_) => Some(Err(InterfaceError::Poison)),
            }),
        }
    }

    pub fn send(
        &self,
        id: Option<StreamId>,
        msg: Box<dyn Message>,
    ) -> Result<StreamId, InterfaceError> {
        let id = match id {
            Some(id) => id,
            None => {
                let inner = self.inner.lock().map_err(|_| InterfaceError::Poison)?;

                // Not certain whether or not strict ordering is required
                let id = inner.stream_counter.fetch_add(1, Ordering::SeqCst);
                StreamId(id)
            }
        };

        match self.inner.lock() {
            Ok(inner) => match inner.outgoing.send((id, msg)) {
                Ok(()) => Ok(id),
                Err(_) => Err(InterfaceError::Disconnected),
            },
            Err(_) => Err(InterfaceError::Poison),
        }
    }

    pub fn close(&self, id: StreamId) -> Result<(), InterfaceError> {
        match self.inner.lock() {
            Ok(inner) => match inner.outgoing.send((id, Box::new(CloseTransaction))) {
                Ok(()) => Ok(()),
                Err(_) => Err(InterfaceError::Disconnected),
            },
            Err(_) => Err(InterfaceError::Poison),
        }
    }
}

impl Clone for Interface {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

#[derive(Debug, Error)]
pub enum InterfaceError {
    #[error("no more messages can be received from the interface")]
    Disconnected,
    #[error("the interface has been poisoned")]
    Poison,
}

pub type WaitHandleResult<Output> = Option<Result<Output, InterfaceError>>;

pub struct WaitHandle<Output> {
    interface: Interface,
    // It would be possible to use non-dynamic types here to improve performance but I couldn't
    // figure out how to do so without mangling all the types and it being really unwieldy
    poll_fn: Box<dyn FnMut(&mut Interface) -> WaitHandleResult<Output>>,
}

impl<Output: 'static> WaitHandle<Output> {
    #[inline]
    pub fn poll(&mut self) -> WaitHandleResult<Output> {
        (self.poll_fn)(&mut self.interface)
    }

    pub fn wait(&mut self) -> Result<Output, InterfaceError> {
        loop {
            if let Some(result) = self.poll() {
                break result;
            }
        }
    }

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
