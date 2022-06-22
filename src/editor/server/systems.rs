use super::EntityCache;
use crate::{
    common::{asynchronous::*, message::*, *},
    editor::server::{generate_self_signed_cert, server_config},
    server_addr,
};
use bevy::prelude::*;
use futures_util::StreamExt;
use quinn::Endpoint;
use std::sync::mpsc::{Receiver, Sender};

pub async fn run_server(
    local_rx: Receiver<(StreamId, Box<dyn Message>)>,
    remote_tx: Sender<(StreamId, Box<dyn Message>)>,
    mut stream_counter: StreamCounter,
) -> Result<(), RemoteThreadError> {
    let (cert, key) = generate_self_signed_cert()?;
    std::fs::write("certificate.der", cert.clone())?;
    let server_config = server_config(cert, key)?;

    let (_endpoint, mut incoming) = Endpoint::server(server_config, server_addr())?;

    println!("Accepting connections!");

    while let Some(conn) = incoming.next().await {
        println!("Waiting for connection...");

        let new = conn.await?;

        println!("Received a connection!");

        process_connection(new, &local_rx, &remote_tx, &mut stream_counter).await?;
    }

    Ok(())
}

pub fn update_entity_cache(
    mut cache: ResMut<EntityCache>,
    mut reader: EventReader<MessageReceived<EntityUpdate>>,
) {
    cache.extend(reader.iter().map(|up| &up.msg.entities).flatten());
}
