use super::resources;
use crate::{common::*, server_addr};
use bevy::{prelude::*, reflect::TypeRegistry};
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

    let registry = world.remove_resource::<TypeRegistry>().expect("failed to get TypeRegistry while starting server thread. Ensure a TypeRegistry is added to the world at startup");
    let server_registry = registry.clone();
    world.insert_resource(registry);

    let server_thread = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        replace_type_registry(server_registry);

        return runtime.block_on(run_server(editor_rx, client_tx));
    });

    world.insert_resource(resources::ServerThread(server_thread));
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
    SendError(#[from] SendError<Box<dyn ClientMessage>>),
    #[error(transparent)]
    SerdeYamlError(#[from] serde_yaml::Error),
    #[error(transparent)]
    StreamWriteError(#[from] WriteError),
    #[error("failed to deserialize message from client")]
    DeserializationFailed(#[from] MessageDeserError),
}

#[derive(Debug, Error)]
pub enum MessageDeserError {
    #[error(transparent)]
    YamlError(#[from] serde_yaml::Error),
    #[error("the received message {} is not registered in the TypeRegistry", .0)]
    MessageNotRegistered(String),
    #[error("the received message {} does not have an accessible FromReflect implementation; make sure to use #[reflect(MessageFromReflect)]", .0)]
    MessageNotFromReflect(String),
    #[error("the received message could not be converted to a concrete type: {:#?}", .0)]
    FromReflectFailed(String),
    #[error("the received message {} does not have an accessible ClientMessage implementation; make sure to use #[reflect(ClientMessage)]", .0)]
    MessageNotClientMessage(String),
}

pub async fn run_server(
    mut editor_rx: Receiver<Box<dyn EditorMessage>>,
    client_tx: Sender<Box<dyn ClientMessage>>,
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

                    let bytes = crate::common::with_type_registry(|reg| {
                        let reg = reg.unwrap().read();

                        let refl = bevy::reflect::serde::ReflectSerializer::new(msg.borrow_reflect(), &*reg);

                        serde_yaml::to_vec(&refl)
                    })?;
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

                    let msg = crate::common::with_type_registry(|reg| {
                        let reg = reg.unwrap().read();

                        let deser = bevy::reflect::serde::ReflectDeserializer::new(&reg);

                        let dynamic = serde_yaml::seed::from_slice_seed(&buf, deser)?;

                        let registration = reg.get_with_name(dynamic.type_name()).ok_or_else(|| MessageDeserError::MessageNotRegistered(dynamic.type_name().into()))?;

                        let from_reflect = registration.data::<ReflectMessageFromReflect>().ok_or_else(|| MessageDeserError::MessageNotFromReflect(dynamic.type_name().into()))?;

                        let msg = from_reflect.from_reflect(&*dynamic).ok_or_else(|| MessageDeserError::FromReflectFailed(String::from_utf8_lossy(&buf).to_string()))?;

                        let reflect_msg = registration.data::<ReflectClientMessage>().ok_or_else(|| MessageDeserError::MessageNotClientMessage(dynamic.type_name().into()))?;

                        let msg = reflect_msg.get_boxed(msg).unwrap();

                        // Type inference died here, not sure why this is necessary
                        Ok::<_, MessageDeserError>(msg)
                    })?;

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
    cache.extend(
        interface
            .collect_messages::<message::EntityUpdate>()
            .into_iter()
            .map(|up| up.entities)
            .flatten(),
    );
}

pub fn monitor_server_thread(world: &mut World) {
    let resources::ServerThread(thread) = world
        .remove_resource::<resources::ServerThread>()
        .expect("ServerThread resource unexpectedly removed");

    if !thread.is_finished() {
        world.insert_resource(resources::ServerThread(thread));
    } else {
        match thread.join() {
            Ok(Ok(())) => {
                eprintln!("Server thread closed normally. Not reopening.");
                return;
            }
            Ok(Err(err)) => eprintln!("Server thread closed with error {err:#?}! Reopening."),
            Err(_) => eprintln!("Server thread closed with an unknown error! Reopening."),
        }

        _ = world.remove_non_send_resource::<resources::ClientInterface>();
        open_server_thread(world);
    }
}
