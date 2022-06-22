use crate::common::message::CloseTransaction;

use super::*;
use bevy::{prelude::*, reflect::TypeRegistry, utils::HashMap};
use futures_util::{select, Future, FutureExt, StreamExt};
use quinn::{
    ConnectError, ConnectionError, NewConnection, ReadExactError, RecvStream, SendStream,
    WriteError,
};
use rcgen::RcgenError;
use std::{
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc::{channel, Receiver, RecvError, SendError, Sender, TryRecvError},
        Arc,
    },
    task::{Context, Poll},
    thread::JoinHandle,
};
use thiserror::Error;

const MAGIC: &[u8; 4] = b"OBRS";

pub struct RemoteThread(pub(crate) JoinHandle<Result<(), RemoteThreadError>>);

#[derive(Debug, Error)]
pub enum RemoteThreadError {
    #[error(transparent)]
    ConnectError(#[from] ConnectError),
    #[error(transparent)]
    ConnectionError(#[from] ConnectionError),
    #[error(transparent)]
    FsWriteError(#[from] std::io::Error),
    #[error(transparent)]
    ProcessConnectionError(#[from] ProcessConnectionError),
    #[error(transparent)]
    ProcessStreamError(#[from] ProcessStreamError),
    #[error(transparent)]
    RcgenError(#[from] RcgenError),
    #[error(transparent)]
    RustlsError(#[from] rustls::Error),
}

pub fn open_remote_thread<F: 'static + Future<Output = Result<(), RemoteThreadError>>>(
    run_fn: impl 'static
        + Fn(
            Receiver<(StreamId, Box<dyn Message>)>,
            Sender<(StreamId, Box<dyn Message>)>,
            Arc<AtomicUsize>,
        ) -> F
        + Send
        + Sync
        + Copy,
) -> impl 'static + Fn(&mut World) {
    move |world| {
        let run_fn = run_fn;

        let (local_tx, local_rx) = channel();
        let (remote_tx, remote_rx) = channel();

        let stream_counter = Arc::new(AtomicUsize::new(0));

        let interface = Interface::new(remote_rx, local_tx, stream_counter.clone());
        world.insert_resource(interface);

        let registry = world.remove_resource::<TypeRegistry>().expect("failed to get TypeRegistry while starting client thread. Ensure a TypeRegistry is added to the world at startup");
        let client_registry = registry.clone();
        world.insert_resource(registry);

        let client_thread = std::thread::spawn(move || {
            let run_fn = run_fn;

            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            _ = replace_type_registry(client_registry);

            runtime.block_on(run_fn(local_rx, remote_tx, stream_counter))
        });

        world.insert_resource(RemoteThread(client_thread));
    }
}

struct PollReceiver<'rx, T> {
    rx: &'rx Receiver<T>,
}

impl<T> Future for PollReceiver<'_, T> {
    type Output = Result<T, RecvError>;

    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        match self.rx.try_recv() {
            Ok(msg) => Poll::Ready(Ok(msg)),
            Err(TryRecvError::Disconnected) => Poll::Ready(Err(RecvError)),
            Err(TryRecvError::Empty) => Poll::Pending,
        }
    }
}

#[derive(Debug, Error)]
pub enum ProcessConnectionError {
    #[error(transparent)]
    ProcessMessageError(#[from] ProcessMessageError),
    #[error(transparent)]
    ProcessStreamError(#[from] ProcessStreamError),
    #[error("interface closed")]
    RecvError(#[from] RecvError),
}

pub async fn process_connection(
    mut new: NewConnection,
    local_rx: &Receiver<(StreamId, Box<dyn Message>)>,
    remote_tx: &Sender<(StreamId, Box<dyn Message>)>,
    stream_counter: &mut Arc<AtomicUsize>,
) -> Result<(), ProcessConnectionError> {
    let mut pool = HashMap::default();
    let mut buffer = vec![0; 1024];

    loop {
        select! {
            stream = new.bi_streams.next().fuse() => {
                println!("Received open stream!");
                process_stream(stream, remote_tx, &mut pool, stream_counter, &mut buffer).await?
            },
            msg = PollReceiver { rx: local_rx }.fuse() => {
                let (id, msg) = msg?;

                process_message(id, msg, &new, &mut pool).await?
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum ProcessStreamError {
    #[error("bi streams closed")]
    BiStreamsClosed,
    #[error(transparent)]
    ConnectionError(#[from] ConnectionError),
    #[error("failed to deserialize message from client")]
    DeserializationFailed(#[from] message::MessageDeserError),
    #[error("received malformed message header {:?}", .0)]
    InvalidData([u8; 4]),
    #[error(transparent)]
    ReadExactError(#[from] ReadExactError),
    #[error(transparent)]
    SendError(#[from] SendError<(StreamId, Box<dyn Message>)>),
}

async fn process_stream(
    stream: Option<Result<(SendStream, RecvStream), ConnectionError>>,
    remote_tx: &Sender<(StreamId, Box<dyn Message>)>,
    pool: &mut HashMap<StreamId, (SendStream, RecvStream)>,
    stream_counter: &mut Arc<AtomicUsize>,
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
    println!("Received message of length {}", len);
    if len > buffer.len() {
        buffer.append(&mut vec![0; len - buffer.len()]);
    }
    let buf = &mut buffer[..len];

    recv.read_exact(buf).await?;
    println!("Read bytes! String value: {}", String::from_utf8_lossy(buf));

    let msg = message::deserialize_message(buf)?;

    println!("Deserialized message: {:?}", msg);

    let stream_id = StreamId(stream_counter.fetch_add(1, Ordering::SeqCst));
    remote_tx.send((stream_id, msg))?;
    pool.insert(stream_id, (send, recv));

    Ok(())
}

#[derive(Debug, Error)]
pub enum ProcessMessageError {
    #[error(transparent)]
    CloseStreamError(#[from] CloseStreamError),
    #[error(transparent)]
    ConnectionError(#[from] ConnectionError),
    #[error(transparent)]
    SerdeYamlError(#[from] serde_yaml::Error),
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
        println!("Closing thread {:?}", id);

        return Ok(close_stream(id, pool).await?);
    }

    println!("Received a message to send! Value: {:?}", msg);
    println!("Sending on stream {:?}", id);

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

    println!(
        "Finished sending a message! Message sent: {:?}",
        String::from_utf8_lossy(&message[..])
    );

    Ok(())
}

#[derive(Debug, Error)]
pub enum CloseStreamError {
    #[error("the interface for closing streams has been unexpectedly closed")]
    CloseChannelClosed(#[from] RecvError),
    #[error(transparent)]
    WriteError(#[from] WriteError),
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

// TODO: Reduce the frequency of this system running
pub fn monitor_remote_thread<F: 'static + Future<Output = Result<(), RemoteThreadError>>>(
    run_fn: impl 'static
        + Fn(
            Receiver<(StreamId, Box<dyn Message>)>,
            Sender<(StreamId, Box<dyn Message>)>,
            Arc<AtomicUsize>,
        ) -> F
        + Send
        + Sync
        + Copy,
) -> impl 'static + Fn(&mut World) {
    move |world| {
        let RemoteThread(thread) = world
            .remove_resource::<RemoteThread>()
            .expect("RemoteThread resource unexpectedly removed");

        if !thread.is_finished() {
            world.insert_resource(RemoteThread(thread));
        } else {
            match thread.join() {
                Ok(Ok(())) => {
                    eprintln!("Remote thread closed normally. Not reopening.");
                    return;
                }
                Ok(Err(err)) => eprintln!("Remote thread closed with error {err:?}! Reopening."),
                Err(_) => eprintln!("Remote thread closed with an unknown error! Reopening."),
            }

            _ = world.remove_resource::<Interface>();
            open_remote_thread(run_fn)(world);
        }
    }
}
