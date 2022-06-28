use std::sync::mpsc::{Receiver, Sender};

use common::asynchronous;
use common::asynchronous::RemoteThreadError;
use common::deps::bevy::prelude::{EventReader, Local, Res, ResMut};
use common::deps::bevy::reflect::Reflect;
use common::deps::bevy::utils::HashMap;
use common::deps::futures_util::StreamExt;
use common::deps::quinn::Endpoint;
use common::interface::{StreamCounter, StreamId};
use common::message::distributor::MessageReceived;
use common::message::messages::{EntityUpdate, SceneDiff};
use common::message::Message;
use common::serde::RemoteEntity;

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

    let (_endpoint, mut incoming) = Endpoint::server(server_config, common::server_addr())?;

    println!("Accepting connections!");

    while let Some(conn) = incoming.next().await {
        println!("Waiting for connection...");

        let new = conn.await?;

        println!("Received a connection!");

        asynchronous::process_connection(new, &local_rx, &remote_tx, &mut stream_counter).await?;
    }

    Ok(())
}

// TODO: If the editor crashes, all scene diffs are lost and future scene diffs will not restore the whole state.
// Add mechanism to refresh by sending the entire scene in this event.
// TODO: If the client crashes, all scene diffs are invalid and future scene diffs will overwrite invalid state.
// Add mechanism to refresh by dumping the entire scene and restoring.
pub(crate) fn apply_scene_diff(
    cache: ResMut<EntityCache>,
    mut reader: EventReader<MessageReceived<SceneDiff>>,
) {
    let mut cache = cache.write().unwrap();
    for diff in reader.iter() {
        for (entity, components) in diff.msg.changes.iter() {
            match cache.get_mut(entity) {
                Some(entity_comps) => entity_comps.extend(
                    components
                        .iter()
                        .cloned()
                        .map(|comp| (comp.type_name().to_string(), comp)),
                ),
                None => {
                    _ = cache.insert(
                        *entity,
                        components
                            .iter()
                            .cloned()
                            .map(|c| (c.type_name().to_string(), c))
                            .collect(),
                    )
                }
            }
        }
    }
}

pub(crate) fn update_entity_cache(// cache: Res<EntityCache>,
    // mut reader: EventReader<MessageReceived<EntityUpdate>>,
    // mut entities: Local<HashMap<RemoteEntity, Option<String>>>,
) {
    // let mut cache = cache.write().unwrap();
    // for update in reader.iter() {
    //     entities.extend(update.msg.entities.iter().cloned());
    //     if update.msg.destroyed {
    //         cache.retain(|entity, _| !entities.contains_key(entity));
    //     } else {
    //         // cache.extend(entities.drain());
    //     }
    // }
}
