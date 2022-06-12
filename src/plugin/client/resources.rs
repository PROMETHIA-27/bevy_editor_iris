use crate::common::{ClientMessage, EditorMessage, Interface};
use bevy::prelude::*;
use std::thread::JoinHandle;

pub type EditorInterface = Interface<EditorMessage, ClientMessage>;

impl EditorInterface {
    pub fn send_entity_update(&mut self, entities: Vec<Entity>) {
        match self.outgoing.blocking_send(ClientMessage::EntityUpdate(
            entities.into_iter().map(|e| e.into()).collect(),
        )) {
            Ok(_) => (),
            Err(_) => {
                eprintln!("could not send update, editor interface thread closed")
            }
        }
    }
}

#[derive(Deref, DerefMut)]
pub struct ClientThread(pub JoinHandle<()>);
