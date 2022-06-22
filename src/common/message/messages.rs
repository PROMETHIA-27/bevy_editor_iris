use crate::common::{
    serde::{ReflectObject, RemoteEntity},
    *,
};
use bevy::{prelude::*, reflect::FromReflect};
use bevy_mod_ouroboros_derive::*;

#[message]
pub struct Ping;

#[message]
pub struct EntityUpdate {
    pub destroyed: bool,
    pub entities: Vec<RemoteEntity>,
}

#[derive(Default)]
#[message]
pub struct ComponentResponse {
    pub components: Vec<Vec<ReflectObject>>,
}

#[derive(Default)]
#[message]
pub struct ComponentQuery {
    pub components: Vec<String>,
    pub entities: Vec<RemoteEntity>,
}

pub struct DefaultMessages;

impl RegisterMessage for DefaultMessages {
    fn register(app: &mut App) {
        Ping::register(app);
        EntityUpdate::register(app);
        ComponentResponse::register(app);
        ComponentQuery::register(app);
    }
}
