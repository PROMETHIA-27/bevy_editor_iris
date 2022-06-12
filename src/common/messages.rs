use super::serde::{ReflectObject, RemoteEntity};
use serde::{Deserialize, Serialize};

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
pub enum EditorMessage {
    ComponentQuery(Vec<String>, Vec<RemoteEntity>),
    Ping,
}

impl Default for EditorMessage {
    fn default() -> Self {
        EditorMessage::Ping
    }
}

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
pub enum ClientMessage {
    ComponentResponse(Vec<Vec<ReflectObject>>),
    EntityUpdate(Vec<RemoteEntity>),
    Ping,
}

impl Default for ClientMessage {
    fn default() -> Self {
        ClientMessage::Ping
    }
}
