use crate::common::*;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc::{Receiver, Sender, TryRecvError},
};
use std::sync::{Arc, Mutex};
use thiserror::Error;

use super::message::CloseTransaction;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct StreamId(pub(crate) usize);

#[derive(Clone)]
pub struct StreamCounter(pub(crate) Arc<AtomicUsize>);

impl Default for StreamCounter {
    fn default() -> Self {
        Self(Arc::new(AtomicUsize::new(0)))
    }
}

impl StreamCounter {
    pub fn next(&self) -> StreamId {
        // Not certain whether or not strict ordering is required
        let id = self.0.fetch_add(1, Ordering::SeqCst);
        StreamId(id)
    }
}

pub(crate) struct InternalInterface {
    pub(crate) incoming: Receiver<(StreamId, Box<dyn Message>)>,
    pub(crate) outgoing: Sender<(StreamId, Box<dyn Message>)>,
}

pub struct Interface {
    pub(crate) inner: Arc<Mutex<InternalInterface>>,
    pub(crate) stream_counter: StreamCounter,
}

impl Interface {
    pub fn new(
        incoming: Receiver<(StreamId, Box<dyn Message>)>,
        outgoing: Sender<(StreamId, Box<dyn Message>)>,
        stream_counter: StreamCounter,
    ) -> Self {
        Self {
            inner: Arc::new(Mutex::new(InternalInterface { outgoing, incoming })),
            stream_counter,
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

    pub fn recv_all(&self) -> Result<Vec<(StreamId, Box<dyn Message>)>, InterfaceError> {
        let inner = self.inner.lock().map_err(|_| InterfaceError::Poison)?;

        Ok(inner.incoming.try_iter().collect())
    }

    pub fn send(
        &self,
        id: Option<StreamId>,
        msg: Box<dyn Message>,
    ) -> Result<StreamId, InterfaceError> {
        let id = match id {
            Some(id) => id,
            None => self.stream_counter.next(),
        };

        match self.inner.lock() {
            Ok(inner) => match inner.outgoing.send((id, msg)) {
                Ok(()) => Ok(id),
                Err(_) => Err(InterfaceError::Disconnected),
            },
            Err(_) => Err(InterfaceError::Poison),
        }
    }

    pub fn send_all(
        &self,
        messages: Vec<(StreamId, Box<dyn Message>)>,
    ) -> Result<(), InterfaceError> {
        let inner = self.inner.lock().map_err(|_| InterfaceError::Poison)?;

        for (id, msg) in messages {
            match inner.outgoing.send((id, msg)) {
                Ok(()) => (),
                Err(_) => return Err(InterfaceError::Disconnected),
            }
        }

        Ok(())
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
            stream_counter: self.stream_counter.clone(),
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
