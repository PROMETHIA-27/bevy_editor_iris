use std::error::Error;
use std::mem;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::thread::JoinHandle;

use bevy::prelude::World;
use bevy::tasks::TaskPool;
use bevy::{reflect::TypeRegistry, utils::HashMap};
use futures::stream::FuturesUnordered;
use futures_lite::{future, Future, FutureExt, StreamExt};
use quinn::{
    ConnectError, ConnectionError, NewConnection, ReadExactError, RecvStream, SendStream,
    WriteError,
};
use rcgen::RcgenError;
use thiserror::Error;
use tokio::select;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::interface::Interface;
use crate::message::messages::CloseTransaction;
use crate::message::{self, MessageDeserError};
use crate::serde;
use crate::Message;

const MAGIC: &[u8; 4] = b"OBRS";

/// A type-erased [Boxed](Box) [message](Message)
pub type MessageBox = Box<dyn Message>;
/// A channel for sending [messages](MessageBox)
pub type MessageTx = UnboundedSender<MessageBox>;
/// A channel for receiving [messages](MessageBox)
pub type MessageRx = UnboundedReceiver<MessageBox>;
/// A channel for sending parts of a bi-directional channel of [messages](MessageBox) between
/// two threads.
pub type OpeningSender = UnboundedSender<(MessageTx, MessageRx)>;
/// A channel for receiving parts of a bi-directional channel of [messages](MessageBox) between
/// two threads.
pub type OpeningReceiver = UnboundedReceiver<(MessageTx, MessageRx)>;

struct ReceiveState {
    recv: RecvStream,
    tx: MessageTx,
    buffer: Vec<u8>,
}
struct SendState {
    send: SendStream,
    rx: MessageRx,
    buffer: Vec<u8>,
}
// TODO: Type-Alias-Impl-Trait might make the Box<Future> unnecessary
type ReceivedMessages =
    FuturesUnordered<Box<dyn Future<Output = Result<(ReceiveState, MessageBox), RecvError>>>>;
type PendingMessages = FuturesUnordered<Box<dyn Future<Output = Result<SendState, SendError>>>>;

/// A [`JoinHandle`] to the remote thread. Used by [`monitor_remote_thread`][crate::systems::monitor_remote_thread] to
/// detect and recover from panics.
pub struct RemoteThread(pub(crate) JoinHandle<Result<(), RemoteThreadError>>);

/// An interface to send and receive messages to/from the remote application
pub struct Transaction {
    tx: MessageTx,
    rx: MessageRx,
}

impl Transaction {
    #[inline]
    fn recv(&mut self) -> Option<MessageBox> {
        self.rx.blocking_recv()
    }

    #[inline]
    fn try_recv(&mut self) -> Result<MessageBox, TryRecvError> {
        self.rx.try_recv()
    }
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
    Other(#[from] Box<dyn Error + Send>),
}

// TODO: Connect to multiple clients?
/// Opens the remote thread using the given run function.
///
/// The remote thread handles transactions between the local threads and the remote
/// application.
pub fn open_remote_thread<F: 'static + Future<Output = Result<(), RemoteThreadError>>>(
    // run_fn: impl 'static + Fn(OpeningSender, OpeningReceiver, StreamCounter) -> F + Send + Sync + Copy,
    run_fn: impl 'static + Fn(OpeningSender, OpeningReceiver) -> F + Send + Sync + Copy,
) -> impl 'static + Fn(&mut World) {
    move |world| {
        let run_fn = run_fn;

        let (remote_tx, local_rx) = mpsc::unbounded_channel();
        let (local_tx, remote_rx) = mpsc::unbounded_channel();

        // let stream_counter = world.remove_resource::<StreamCounter>().expect("failed to get StreamCounter while starting remote thread. Ensure a StreamCounter is added to the world at startup");
        // let thread_counter = stream_counter.clone();

        // let interface = Interface::new(local_tx, local_rx, stream_counter.clone());
        let interface = Interface::new(local_tx, local_rx);
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

            // runtime.block_on(run_fn(remote_tx, remote_rx, thread_counter))
            runtime.block_on(run_fn(remote_tx, remote_rx))
        });

        // world.insert_resource(stream_counter);
        world.insert_resource(RemoteThread(client_thread));
    }
}

/// A temporary hack future which allows polling std::sync::mpsc channels with async.
/// Not intended to last forever, only while I get around to replacing these channels
/// with tokio::sync::mpsc.
// struct PollReceiver<'rx, T> {
//     rx: &'rx UnboundedReceiver<T>,
// }

// impl<T> Future for PollReceiver<'_, T> {
//     type Output = Result<T, TryRecvError>;

//     fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
//         // TODO: This is inefficient and a big waste of processing power.
//         // Switch to tokio's mpsc to send messages to the remote thread,
//         // and switch to tokio's broadcast to send messages to the local thread.
//         // Questions for doing that; channels are not sync, but they are send.
//         // How to give each system a local receiver/sender?
//         ctx.waker().wake_by_ref();
//         match self.rx.try_recv() {
//             Ok(msg) => Poll::Ready(Ok(msg)),
//             Err(TryRecvError::Disconnected) => Poll::Ready(Err(TryRecvError)),
//             Err(TryRecvError::Empty) => Poll::Pending,
//         }
//     }
// }

/// An error that occurs when processing an incoming connection.
#[derive(Debug, Error)]
pub enum ProcessConnectionError {
    /// An error occurred while processing a [`Message`] to send.
    #[error(transparent)]
    ProcessMessageError(#[from] ProcessMessageError),
    /// An error occurred while processing an incoming stream.
    #[error(transparent)]
    ProcessStreamError(#[from] ProcessStreamError),
    // /// The [`Interface`] closed unexpectedly.
    // #[error("interface closed")]
    // RecvError(#[from] RecvError),
}

/// Processes incoming transactions and messages to send, sending messages
/// between the two given channels.
pub async fn process_connection(
    mut new: NewConnection,
    tx: &OpeningSender,
    rx: &OpeningReceiver,
    // stream_counter: &mut StreamCounter,
) -> Result<(), ProcessConnectionError> {
    // let mut pool = HashMap::default();
    let mut buffer = vec![0; 1024];

    let mut pending_messages = FuturesUnordered::new();
    let mut received_messages = FuturesUnordered::new();

    loop {
        select! {
            stream = new.bi_streams.next() => {
                // process_incoming_bi(stream, tx, &mut pool, stream_counter, &mut received_messages, &mut pending_messages).await?
                process_incoming_bi(stream, tx, &mut received_messages, &mut pending_messages).await?
            },
            // msg = PollReceiver { rx: local_rx } => {
            //     let (id, msg) = msg?;

            //     process_message(id, msg, &new, &mut pool).await?
            // }
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
    Connection(#[from] ConnectionError),
    /// Failed to send a message to the local threads.
    #[error(transparent)]
    Send(#[from] tokio::sync::mpsc::error::SendError<(MessageTx, MessageRx)>),
    /// Failed to receive a message from the remote application
    #[error(transparent)]
    Recv(#[from] RecvError),
}

async fn process_incoming_bi(
    stream: Option<Result<(SendStream, RecvStream), ConnectionError>>,
    open_tx: &OpeningSender,
    // pool: &mut HashMap<StreamId, (SendStream, RecvStream)>,
    // stream_counter: &mut StreamCounter,
    received_messages: &mut ReceivedMessages,
    pending_messages: &mut PendingMessages,
) -> Result<(), ProcessStreamError> {
    let stream = stream.ok_or_else(|| ProcessStreamError::BiStreamsClosed)?;
    let (send, mut recv) = stream?;

    let (tx, local_rx) = mpsc::unbounded_channel();
    let (local_tx, rx) = mpsc::unbounded_channel();

    open_tx.send((local_tx, local_rx))?;
    received_messages.push(Box::new(receive_message(ReceiveState {
        recv,
        tx,
        buffer: vec![],
    })));

    pending_messages.push(Box::new(send_message(SendState {
        send,
        rx,
        buffer: vec![],
    })));

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
    /// Failed to write to the remote stream.
    #[error(transparent)]
    StreamWriteError(#[from] WriteError),
}

// async fn process_message(
//     id: StreamId,
//     msg: Box<dyn Message>,
//     new: &NewConnection,
//     pool: &mut HashMap<StreamId, (SendStream, RecvStream)>,
// ) -> Result<(), ProcessMessageError> {
// }

/// An error that occurs while attempting to close a stream.
#[derive(Debug, Error)]
pub enum CloseStreamError {
    // /// The interface unexpectedly closed.
    // #[error("the interface unexpectedly closed")]
    // CloseChannelClosed(#[from] RecvError),
    /// Attempted to close a stream which does not exist.
    #[error("stream does not exist")]
    DoesNotExist,
    /// Failed to write to the remote stream.
    #[error(transparent)]
    WriteError(#[from] WriteError),
}

// async fn close_stream(
//     id: StreamId,
//     pool: &mut HashMap<StreamId, (SendStream, RecvStream)>,
// ) -> Result<(), CloseStreamError> {
//     let (mut send, _) = match pool.remove(&id) {
//         Some(stream) => stream,
//         None => return Err(CloseStreamError::DoesNotExist),
//     };

//     send.finish().await?;

//     Ok(())
// }

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
}

async fn receive_message(
    ReceiveState {
        mut recv,
        tx,
        mut buffer,
    }: ReceiveState,
) -> Result<(ReceiveState, MessageBox), RecvError> {
    let mut header = [0; 12];
    recv.read_exact(&mut header).await?;
    if header[0..4] != *MAGIC {
        let mut arr = [0; 4];
        arr.copy_from_slice(&header[0..4]);
        return Err(RecvError::InvalidData(arr));
    }

    let len = usize::from_le_bytes(header[4..12].try_into().unwrap());
    if len > buffer.len() {
        buffer.append(&mut vec![0; len - buffer.len()]);
    }
    let buf = &mut buffer[..len];

    recv.read_exact(buf).await?;

    let msg = message::deserialize_message(buf)?;

    Ok((ReceiveState { recv, tx, buffer }, msg))
}

/// An error that occurs when trying to send a message to the remote application
#[derive(Debug, Error)]
pub enum SendError {
    /// The message channel for this transaction was closed prematurely
    #[error("the message channel between this transaction and the local thread is closed")]
    ChannelClosed,
    /// An error occurred while serializing the message.
    #[error(transparent)]
    SerdeYamlError(#[from] serde_yaml::Error),
    /// The local thread sent a signal to close this transaction, and it was closed.
    /// This error should always be recovered from, as it indicates normal operations.
    #[error("the local thread closed this transaction")]
    TransactionClosed,
    /// Failed to write to the remote stream.
    #[error(transparent)]
    WriteError(#[from] WriteError),
}

async fn send_message(
    SendState {
        mut send,
        mut rx,
        mut buffer,
    }: SendState,
) -> Result<SendState, SendError> {
    let msg = match rx.recv().await {
        Some(m) => m,
        None => return Err(SendError::ChannelClosed),
    };

    if msg.as_any().is::<CloseTransaction>() {
        return Err(SendError::TransactionClosed);
    }

    // For clarity:
    // create a header of [MAGIC, 0usize], write the payload to the message,
    // then go back and write the payload length to the 0'd part of the header.
    const HEADER_SIZE: usize = MAGIC.len() + mem::size_of::<usize>();
    buffer.extend_from_slice(MAGIC);
    buffer.extend_from_slice(&usize::to_le_bytes(0));
    message::serialize_message(msg, &mut buffer)?;
    let message_len = buffer.len();
    buffer[MAGIC.len()..HEADER_SIZE]
        .copy_from_slice(&usize::to_le_bytes(message_len - HEADER_SIZE));

    send.write_all(&buffer).await?;

    Ok(SendState { send, rx, buffer })
}
