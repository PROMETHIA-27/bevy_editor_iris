use crate::common::{ClientMessage, EditorMessage, Interface};
use bevy::prelude::*;
use std::thread::JoinHandle;
use tokio::sync::mpsc::error::SendError;

pub type EditorInterface = Interface<EditorMessage, ClientMessage>;

impl EditorInterface {
    pub fn send_entity_update(
        &mut self,
        entities: Vec<Entity>,
    ) -> Result<(), SendError<ClientMessage>> {
        self.outgoing.blocking_send(ClientMessage::EntityUpdate(
            entities.into_iter().map(|e| e.into()).collect(),
        ))
    }
}

#[derive(Deref, DerefMut)]
pub struct ClientThread(pub JoinHandle<()>);
