use bevy::prelude::{App, Reflect};
use bevy::reflect::FromReflect;
use bevy_editor_iris_derive::{message, Message};

use crate::message::distributor::RegisterMessage;
use crate::message::{Message, ReflectMessage, ReflectMessageFromReflect};
use crate::serde::{ReflectObject, RemoteEntity};

/// Represents a ping message, which can be used for debugging
#[message]
pub struct Ping;

// TODO: Decide whether or not to split this into "NewEntities" and "DestroyedEntities"
// TODO: Should this send full entity data? Without that, it's difficult to actually request all component data
// to display in the inspector.
// TODO: This should be a `HashMap<RemoteEntity, Option<String>>`, but due to reflection serialization limitations
// it must not be a `Hash___` type
/// A message to inform the editor of created or destroyed entities
#[message]
pub struct EntityUpdate {
    /// Whether this set of entities is destroyed or created
    pub destroyed: bool,
    /// The set of entities and their names, if they have one
    pub entities: Vec<(RemoteEntity, Option<String>)>,
}

/// A special-case message which will close a transaction on the local end of the connection
/// if sent like a normal message.
#[message]
pub struct CloseTransaction;

/// TODO: Drop change-detection updates in favor of a query architecture
/// A message to inform the editor of changes in the components of a scene
#[message]
pub struct SceneDiff {
    /// The set of entities and their components which have changed since last diff
    pub changes: Vec<(RemoteEntity, Vec<ReflectObject>)>,
}

/// A registerable set of built-in messages to the iris editor.
pub struct DefaultMessages;

impl RegisterMessage for DefaultMessages {
    fn register(app: &mut App) {
        Ping::register(app);
        EntityUpdate::register(app);
        CloseTransaction::register(app);
        SceneDiff::register(app);
    }
}
