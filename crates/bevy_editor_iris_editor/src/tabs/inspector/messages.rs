use crate::deps::common::deps::bevy::reflect as bevy_reflect;
use bevy_reflect::{FromReflect, Reflect};
use common::message::{Message, ReflectMessage, ReflectMessageFromReflect};
use common::serde::RemoteEntity;
use derive::{message, Message};

#[message]
pub struct ComponentQuery {
    pub entity: RemoteEntity,
}

/// Tells the editor that the following entity data the client sends will be the data of `entity`.
#[message]
pub struct SendingEntityData {
    pub entity: RemoteEntity,
}
