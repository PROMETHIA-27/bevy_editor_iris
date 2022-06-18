use crate::common::*;
use bevy::prelude::*;
use std::thread::JoinHandle;
use tokio::sync::mpsc::error::SendError;

pub type EditorInterface = Interface<Box<dyn EditorMessage>, Box<dyn ClientMessage>>;

impl EditorInterface {
    pub fn send_entity_update(
        &mut self,
        entities: Vec<Entity>,
    ) -> Result<(), SendError<Box<dyn ClientMessage>>> {
        self.outgoing.blocking_send(Box::new(message::EntityUpdate {
            entities: entities.into_iter().map(|e| e.into()).collect(),
        }))
    }
}

#[derive(Deref, DerefMut)]
pub struct ClientThread(pub JoinHandle<()>);
