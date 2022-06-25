use bevy::prelude::{App, Reflect};
use bevy::reflect::{FromReflect};
use bevy::utils::HashMap;
use bevy_mod_ouroboros_derive::{message, Message};

use crate::message::{Message, ReflectMessage, ReflectMessageFromReflect, RegisterMessage};
use crate::serde::{ReflectObject, RemoteEntity};

#[message]
pub struct Ping;

// TODO: Decide whether or not to split this into "NewEntities" and "DestroyedEntities"
// TODO: Should this send full entity data? Without that, it's difficult to actually request all component data
// to display in the inspector.
// TODO: This should be a `HashMap<RemoteEntity, Option<String>>`, but due to reflection serialization limitations
// it must not be a `Hash___` type
#[message]
pub struct EntityUpdate {
    pub destroyed: bool,
    pub entities: Vec<(RemoteEntity, Option<String>)>,
}

#[derive(Default)]
#[message]
pub struct ComponentResponse {
    pub components: HashMap<RemoteEntity, HashMap<String, ReflectObject>>,
}

#[derive(Default)]
#[message]
pub struct ComponentQuery {
    pub components: Vec<String>,
    pub entities: Vec<RemoteEntity>,
}

#[message]
pub struct CloseTransaction;

#[message]
pub struct SceneDiff {
    pub changes: Vec<(RemoteEntity, Vec<ReflectObject>)>,
}

pub struct DefaultMessages;

impl RegisterMessage for DefaultMessages {
    fn register(app: &mut App) {
        Ping::register(app);
        EntityUpdate::register(app);
        ComponentResponse::register(app);
        ComponentQuery::register(app);
        CloseTransaction::register(app);
        SceneDiff::register(app);
    }
}
