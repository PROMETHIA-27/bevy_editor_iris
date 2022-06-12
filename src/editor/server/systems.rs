use super::resources;
use crate::{
    common::{ClientMessage, EditorMessage},
    server_addr,
};
use bevy::prelude::*;
use futures_util::{select, FutureExt, StreamExt};
use quinn::{ConnectionError, Endpoint, ReadExactError, WriteError};
use rcgen::RcgenError;
use thiserror::Error;
use tokio::sync::mpsc::{channel, error::SendError, Receiver, Sender};

pub fn open_server_thread(world: &mut World) {
    // 128 buffer chosen arbitrarily; Feel free to discuss whether this should be changed
    let (editor_tx, editor_rx) = channel(128);
    let (client_tx, client_rx) = channel(128);

    let interface = resources::ClientInterface::new(editor_tx, client_rx);
    world.insert_non_send_resource(interface);

    let _server_thread = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        runtime.block_on(run_server(editor_rx, client_tx)).unwrap();
    });
}

const MAGIC: &[u8; 4] = b"OBRS";

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum RunServerError {
    #[error(transparent)]
    ConnectionError(#[from] ConnectionError),
    #[error(transparent)]
    FsWriteError(#[from] std::io::Error),
    #[error(transparent)]
    RcgenError(#[from] RcgenError),
    #[error(transparent)]
    ReadExactError(#[from] ReadExactError),
    #[error(transparent)]
    RustlsError(#[from] rustls::Error),
    #[error(transparent)]
    SendError(#[from] SendError<ClientMessage>),
    #[error(transparent)]
    SerdeYamlError(#[from] serde_yaml::Error),
    #[error(transparent)]
    StreamWriteError(#[from] WriteError),
}

pub async fn run_server(
    mut editor_rx: Receiver<EditorMessage>,
    client_tx: Sender<ClientMessage>,
) -> Result<(), RunServerError> {
    let (cert, key) = super::generate_self_signed_cert()?;
    std::fs::write("certificate.der", cert.clone())?;
    let server_config = super::server_config(cert, key)?;

    let (_endpoint, mut incoming) = Endpoint::server(server_config, server_addr())?;

    println!("Accepting connections!");

    while let Some(conn) = incoming.next().await {
        println!("Waiting for connection...");

        let new = conn.await?;

        println!("Received a connection!");

        let (mut send, mut recv) = new.connection.open_bi().await?;

        let mut header = [0; 12];
        'read: loop {
            select! {
                msg = editor_rx.recv().fuse() => {
                    let msg = if let Some(msg) = msg {
                        msg
                    } else {
                        break 'read;
                    };
                    let bytes = serde_yaml::to_vec(&msg)?;
                    let mut header = [0; 12];
                    header[0..4].copy_from_slice(MAGIC);
                    header[4..12].copy_from_slice(&usize::to_le_bytes(bytes.len()));
                    send.write(&header).await?;
                    send.write(&bytes).await?;
                },
                msg = recv.read_exact(&mut header).fuse() => {
                    msg?;

                    if header[0..4] != *MAGIC {
                        panic!("Received invalid data");
                    }

                    let len = usize::from_le_bytes(header[4..12].try_into().unwrap());
                    let mut buf = Vec::with_capacity(len);
                    _ = recv.read_exact(&mut buf).await?;

                    let msg: ClientMessage = serde_yaml::from_slice(&buf)?;

                    client_tx.send(msg).await?;
                },
            }
        }
    }

    Ok(())
}

pub fn update_entity_cache(
    mut cache: ResMut<resources::EntityCache>,
    mut interface: NonSendMut<resources::ClientInterface>,
) {
    cache.extend(interface.collect_entity_updates().into_iter().flatten());
}
