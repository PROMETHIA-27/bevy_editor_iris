use std::sync::mpsc::{Receiver, Sender};

use bevy_editor_iris_common::asynchronous::RemoteThreadError;
use bevy_editor_iris_common::bevy::prelude::{EventReader, Local, Res};
use bevy_editor_iris_common::bevy::utils::HashMap;
use bevy_editor_iris_common::futures_util::StreamExt;
use bevy_editor_iris_common::message::{EntityUpdate, SceneDiff};
use bevy_editor_iris_common::quinn::Endpoint;
use bevy_editor_iris_common::{
    asynchronous, Message, MessageReceived, ReflectObject, RemoteEntity, StreamCounter, StreamId,
};

use crate::server;

use super::EntityCache;

pub async fn run_server(
    local_rx: Receiver<(StreamId, Box<dyn Message>)>,
    remote_tx: Sender<(StreamId, Box<dyn Message>)>,
    mut stream_counter: StreamCounter,
) -> Result<(), RemoteThreadError> {
    let (cert, key) = server::generate_self_signed_cert()?;
    std::fs::write("certificate.der", cert.clone())?;
    let server_config = server::server_config(cert, key)?;

    let (_endpoint, mut incoming) =
        Endpoint::server(server_config, bevy_editor_iris_common::server_addr())?;

    println!("Accepting connections!");

    while let Some(conn) = incoming.next().await {
        println!("Waiting for connection...");

        let new = conn.await?;

        println!("Received a connection!");

        asynchronous::process_connection(new, &local_rx, &remote_tx, &mut stream_counter).await?;
    }

    Ok(())
}

pub(crate) fn apply_scene_diff(
    cache: Res<EntityCache>,
    mut reader: EventReader<MessageReceived<SceneDiff>>,
) {
    let mut cache = cache.write().unwrap();
    for diff in reader.iter() {
        for (entity, components) in diff.msg.changes.iter() {
            match cache.get_mut(entity) {
                Some(entity_comps) => entity_comps.extend(components.iter().cloned()),
            }
        }
        cache.extend(diff.msg.changes.iter().cloned());
    }
}

pub(crate) fn update_entity_cache(
    cache: Res<EntityCache>,
    mut reader: EventReader<MessageReceived<EntityUpdate>>,
    mut entities: Local<HashMap<RemoteEntity, Option<String>>>,
) {
    let mut cache = cache.write().unwrap();
    for update in reader.iter() {
        entities.extend(update.msg.entities.iter().cloned());
        if update.msg.destroyed {
            cache.retain(|entity, _| !entities.contains_key(entity));
        } else {
            cache.extend(entities.drain());
        }
    }
}
