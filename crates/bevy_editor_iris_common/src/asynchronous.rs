use std::error::Error;
use std::pin::Pin;
use std::sync::mpsc::{self, Receiver, RecvError, SendError, Sender, TryRecvError};
use std::task::{Context, Poll};
use std::thread::JoinHandle;

use bevy::prelude::World;
use bevy::{reflect::TypeRegistry, utils::HashMap};
use futures_util::{select, Future, FutureExt, StreamExt};
use quinn::{
    ConnectError, ConnectionError, NewConnection, ReadExactError, RecvStream, SendStream,
    WriteError,
};
use rcgen::RcgenError;
use thiserror::Error;

use crate::interface::{Interface, StreamCounter};
use crate::message::messages::CloseTransaction;
use crate::message::{self, MessageDeserError};
use crate::serde;
use crate::{Message, StreamId};

const MAGIC: &[u8; 4] = b"OBRS";

// TODO: Use this instead of sending through the interface
// pub struct MessageSender(tokio::sync::mpsc::Sender<(StreamId, Box<dyn Message>)>);

/// A [`JoinHandle`] to the remote thread. Used by [`crate::systems::monitor_remote_thread()`] to
/// detect and recover from panics.
pub struct RemoteThread(pub(crate) JoinHandle<Result<(), RemoteThreadError>>);

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
    Other(#[from] Box<dyn Error + Send>),
}

// TODO: Connect to multiple clients?
/// Opens the remote thread using the given run function.
///
/// The remote thread handles transactions between the local threads and the remote
/// application.
pub fn open_remote_thread<F: 'static + Future<Output = Result<(), RemoteThreadError>>>(
    run_fn: impl 'static
        + Fn(
            Receiver<(StreamId, Box<dyn Message>)>,
            Sender<(StreamId, Box<dyn Message>)>,
            StreamCounter,
        ) -> F
        + Send
        + Sync
        + Copy,
) -> impl 'static + Fn(&mut World) {
    move |world| {
        let run_fn = run_fn;

        let (local_tx, local_rx) = mpsc::channel();
        let (remote_tx, remote_rx) = mpsc::channel();

        let stream_counter = world.remove_resource::<StreamCounter>().expect("failed to get StreamCounter while starting remote thread. Ensure a StreamCounter is added to the world at startup");
        let thread_counter = stream_counter.clone();

        let interface = Interface::new(remote_rx, local_tx, stream_counter.clone());
        world.insert_resource(interface);

        let registry = world.remove_resource::<TypeRegistry>().expect("failed to get TypeRegistry while starting remote thread. Ensure a TypeRegistry is added to the world at startup");
        let client_registry = registry.clone();
        world.insert_resource(registry);

        let client_thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            // TODO: Should the type registry be deep cloned instead of arc cloned?
            _ = serde::replace_type_registry(client_registry);

            runtime.block_on(run_fn(local_rx, remote_tx, thread_counter))
        });

        world.insert_resource(stream_counter);
        world.insert_resource(RemoteThread(client_thread));
    }
}

/// A temporary hack future which allows polling std::sync::mpsc channels with async.
/// Not intended to last forever, only while I get around to replacing these channels
/// with tokio::sync::mpsc.
struct PollReceiver<'rx, T> {
    rx: &'rx Receiver<T>,
}

impl<T> Future for PollReceiver<'_, T> {
    type Output = Result<T, RecvError>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
        // TODO: This is inefficient and a big waste of processing power.
        // Switch to tokio's mpsc to send messages to the remote thread,
        // and switch to tokio's broadcast to send messages to the local thread.
        // Questions for doing that; channels are not sync, but they are send.
        // How to give each system a local receiver/sender?
        ctx.waker().wake_by_ref();
        match self.rx.try_recv() {
            Ok(msg) => Poll::Ready(Ok(msg)),
            Err(TryRecvError::Disconnected) => Poll::Ready(Err(RecvError)),
            Err(TryRecvError::Empty) => Poll::Pending,
        }
    }
}

/// An error that occurs when processing an incoming connection.
#[derive(Debug, Error)]
pub enum ProcessConnectionError {
    /// An error occurred while processing a [`Message`] to send.
    #[error(transparent)]
    ProcessMessageError(#[from] ProcessMessageError),
    /// An error occurred while processing an incoming stream.
    #[error(transparent)]
    ProcessStreamError(#[from] ProcessStreamError),
    /// The [`Interface`] closed unexpectedly.
    #[error("interface closed")]
    RecvError(#[from] RecvError),
}

/// Processes incoming transactions and messages to send, sending messages
/// between the two given channels.
pub async fn process_connection(
    mut new: NewConnection,
    local_rx: &Receiver<(StreamId, Box<dyn Message>)>,
    remote_tx: &Sender<(StreamId, Box<dyn Message>)>,
    stream_counter: &mut StreamCounter,
) -> Result<(), ProcessConnectionError> {
    let mut pool = HashMap::default();
    let mut buffer = vec![0; 1024];

    loop {
        select! {
            stream = new.bi_streams.next().fuse() => {
                process_stream(stream, remote_tx, &mut pool, stream_counter, &mut buffer).await?
            },
            msg = PollReceiver { rx: local_rx }.fuse() => {
                let (id, msg) = msg?;

                process_message(id, msg, &new, &mut pool).await?
            }
        }
    }
}

/// An error that occurs when processing an incoming stream.
#[derive(Debug, Error)]
pub enum ProcessStreamError {
    /// The stream of bi-streams unexpectedly closed.
    #[error("bi streams closed")]
    BiStreamsClosed,
    /// The connection unexpectedly closed.
    #[error(transparent)]
    ConnectionError(#[from] ConnectionError),
    /// Message deserialization failed.
    #[error("failed to deserialize message from client")]
    DeserializationFailed(#[from] MessageDeserError),
    /// The message header was invalid, indicating corrupt or malicious
    /// data is being sent.
    #[error("received malformed message header {:?}", .0)]
    InvalidData([u8; 4]),
    /// The stream unexpectedly closed before all data could be received.
    #[error(transparent)]
    ReadExactError(#[from] ReadExactError),
    /// Failed to send a message to the local threads.
    #[error(transparent)]
    SendError(#[from] SendError<(StreamId, Box<dyn Message>)>),
}

async fn process_stream(
    stream: Option<Result<(SendStream, RecvStream), ConnectionError>>,
    remote_tx: &Sender<(StreamId, Box<dyn Message>)>,
    pool: &mut HashMap<StreamId, (SendStream, RecvStream)>,
    stream_counter: &mut StreamCounter,
    buffer: &mut Vec<u8>,
) -> Result<(), ProcessStreamError> {
    let stream = stream.ok_or_else(|| ProcessStreamError::BiStreamsClosed)?;
    let (send, mut recv) = stream?;

    let mut header = [0; 12];
    recv.read_exact(&mut header).await?;
    if header[0..4] != *MAGIC {
        let mut arr = [0; 4];
        arr.copy_from_slice(&header[0..4]);
        return Err(ProcessStreamError::InvalidData(arr));
    }

    let len = usize::from_le_bytes(header[4..12].try_into().unwrap());
    if len > buffer.len() {
        buffer.append(&mut vec![0; len - buffer.len()]);
    }
    let buf = &mut buffer[..len];

    recv.read_exact(buf).await?;

    let msg = message::deserialize_message(buf)?;

    let stream_id = stream_counter.next();
    remote_tx.send((stream_id, msg))?;
    pool.insert(stream_id, (send, recv));

    Ok(())
}

/// An error that occurs while processing a message to send.
#[derive(Debug, Error)]
pub enum ProcessMessageError {
    /// An error occurred while closing a stream.
    #[error(transparent)]
    CloseStreamError(#[from] CloseStreamError),
    /// The connection was unexpectedly lost.
    #[error(transparent)]
    ConnectionError(#[from] ConnectionError),
    /// An error occurred while serializing the message.
    #[error(transparent)]
    SerdeYamlError(#[from] serde_yaml::Error),
    /// Failed to write to the remote stream.
    #[error(transparent)]
    StreamWriteError(#[from] WriteError),
}

async fn process_message(
    id: StreamId,
    msg: Box<dyn Message>,
    new: &NewConnection,
    pool: &mut HashMap<StreamId, (SendStream, RecvStream)>,
) -> Result<(), ProcessMessageError> {
    if msg.as_any().is::<CloseTransaction>() {
        return Ok(close_stream(id, pool).await?);
    }

    let bytes = message::serialize_message(msg)?;
    let mut message = Vec::with_capacity(12 + bytes.len());
    message.extend_from_slice(MAGIC);
    message.extend_from_slice(&usize::to_le_bytes(bytes.len()));
    message.extend_from_slice(&bytes[..]);

    if let Some((send, _)) = pool.get_mut(&id) {
        send.write_all(&message[..]).await?;
    } else {
        let (mut send, recv) = new.connection.open_bi().await?;

        send.write_all(&message[..]).await?;

        pool.insert(id, (send, recv));
    };

    Ok(())
}

/// An error that occurs while attempting to close a stream.
#[derive(Debug, Error)]
pub enum CloseStreamError {
    /// The interface unexpectedly closed.
    #[error("the interface unexpectedly closed")]
    CloseChannelClosed(#[from] RecvError),
    /// Failed to write to the remote stream.
    #[error(transparent)]
    WriteError(#[from] WriteError),
    /// Attempted to close a stream which does not exist.
    #[error("stream does not exist")]
    DoesNotExist,
}

async fn close_stream(
    id: StreamId,
    pool: &mut HashMap<StreamId, (SendStream, RecvStream)>,
) -> Result<(), CloseStreamError> {
    let (mut send, _) = match pool.remove(&id) {
        Some(stream) => stream,
        None => return Err(CloseStreamError::DoesNotExist),
    };

    send.finish().await?;

    Ok(())
}
