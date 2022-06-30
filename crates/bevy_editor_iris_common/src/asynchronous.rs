use std::error::Error;
use std::mem;
use std::pin::Pin;
use std::thread::JoinHandle;

use bevy::prelude::World;
use bevy::reflect::TypeRegistry;
use futures::stream::FuturesUnordered;
use futures_lite::{Future, StreamExt};
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
// TODO: Type-Alias-Impl-Trait might make the Pin<Box<Future>> unnecessary
type ReceivedMessages =
    FuturesUnordered<Pin<Box<dyn Future<Output = Result<ReceiveState, RecvError>>>>>;
type PendingMessages =
    FuturesUnordered<Pin<Box<dyn Future<Output = Result<SendState, SendError>>>>>;

/// A [`JoinHandle`] to the remote thread. Used by [`monitor_remote_thread`][crate::systems::monitor_remote_thread] to
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
    // run_fn: impl 'static + Fn(OpeningSender, OpeningReceiver, StreamCounter) -> F + Send + Sync + Copy,
    run_fn: impl 'static + Fn(OpeningSender, OpeningReceiver) -> F + Send + Sync + Copy,
) -> impl 'static + Fn(&mut World) {
    move |world| {
        let run_fn = run_fn;

        let (remote_tx, local_rx) = mpsc::unbounded_channel();
        let (local_tx, remote_rx) = mpsc::unbounded_channel();

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

            runtime.block_on(run_fn(remote_tx, remote_rx))
        });

        world.insert_resource(RemoteThread(client_thread));
    }
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

/// Processes incoming transactions and messages to send, sending messages
/// between the two given channels.
pub async fn process_connection(
    mut new: NewConnection,
    tx: &OpeningSender,
    rx: &mut OpeningReceiver,
    // stream_counter: &mut StreamCounter,
) -> Result<(), ProcessConnectionError> {
    let mut pending_messages = FuturesUnordered::new();
    let mut received_messages = FuturesUnordered::new();

    loop {
        select! {
            // The remote application opened a new stream
            stream = new.bi_streams.next() => {
                // process_incoming_bi(stream, tx, &mut pool, stream_counter, &mut received_messages, &mut pending_messages).await?
                process_incoming_bi(stream, tx, &mut received_messages, &mut pending_messages).await?
            },
            // The local thread(s) opened a new channel
            channel = rx.recv() => {
                process_incoming_channel(channel, &new, &mut received_messages, &mut pending_messages).await?
            }
            // The local thread(s) sent a new message
            pending = pending_messages.next() => {
                if let Some(pending) = pending {
                    match pending {
                        Ok(SendState { send, rx, buffer }) => {
                            setup_pending(send, rx, buffer, &mut pending_messages);
                        },
                        Err(SendError::TransactionClosed) => (),
                        Err(err) => eprintln!("Send stream closed with error {:?}", err),
                    }
                }
            }
            // The remote application sent us a message
            received = received_messages.next() => {
                if let Some(received) = received {
                    match received {
                        Ok(ReceiveState { recv, tx, buffer }) => {
                            setup_received(recv, tx, buffer, &mut received_messages);
                        },
                        Err(err) => eprintln!("Recv stream closed with error {:?}", err),
                    }
                }
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
    Connection(#[from] ConnectionError),
    /// Failed to send a message to the local threads.
    #[error(transparent)]
    Send(#[from] tokio::sync::mpsc::error::SendError<(MessageTx, MessageRx)>),
    /// Failed to receive a message from the remote application.
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
    let (send, recv) = stream?;

    let (tx, local_rx) = mpsc::unbounded_channel();
    let (local_tx, rx) = mpsc::unbounded_channel();

    open_tx.send((local_tx, local_rx))?;

    setup_message_listeners(
        send,
        recv,
        tx,
        rx,
        vec![],
        vec![],
        pending_messages,
        received_messages,
    );

    Ok(())
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

async fn process_incoming_channel(
    channel: Option<(MessageTx, MessageRx)>,
    new: &NewConnection,
    received_messages: &mut ReceivedMessages,
    pending_messages: &mut PendingMessages,
) -> Result<(), ProcessChannelError> {
    let (tx, rx) = match channel {
        Some(channel) => channel,
        None => return Err(ProcessChannelError::OpenChannelClosed),
    };

    let (send, recv) = new.connection.open_bi().await?;

    setup_message_listeners(
        send,
        recv,
        tx,
        rx,
        vec![],
        vec![],
        pending_messages,
        received_messages,
    );

    Ok(())
}

fn setup_message_listeners(
    send: SendStream,
    recv: RecvStream,
    tx: MessageTx,
    rx: MessageRx,
    pend_buffer: Vec<u8>,
    recv_buffer: Vec<u8>,
    pending_messages: &mut PendingMessages,
    received_messages: &mut ReceivedMessages,
) {
    setup_received(recv, tx, recv_buffer, received_messages);

    setup_pending(send, rx, pend_buffer, pending_messages);
}

fn setup_received(
    recv: RecvStream,
    tx: MessageTx,
    buffer: Vec<u8>,
    received_messages: &mut ReceivedMessages,
) {
    received_messages.push(Box::pin(receive_message(ReceiveState { recv, tx, buffer })));
}

fn setup_pending(
    send: SendStream,
    rx: MessageRx,
    buffer: Vec<u8>,
    pending_messages: &mut PendingMessages,
) {
    pending_messages.push(Box::pin(send_message(SendState { send, rx, buffer })));
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

async fn receive_message(
    ReceiveState {
        mut recv,
        tx,
        mut buffer,
    }: ReceiveState,
) -> Result<ReceiveState, RecvError> {
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

    tx.send(msg)?;

    Ok(ReceiveState { recv, tx, buffer })
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
