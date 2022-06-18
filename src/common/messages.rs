use crate::common::{
    serde::{ReflectObject, RemoteEntity},
    *,
};
use bevy::{
    prelude::*,
    reflect::{FromReflect, TypeRegistry},
};
use bevy_mod_ouroboros_derive::*;

pub fn register_messages(registry: ResMut<TypeRegistry>) {
    let mut registry = registry.write();

    macro_rules! register {
        ($($ty:path),* $(,)?) => {
            $(
                registry.register::<$ty>();
            )*
        }
    }

    register![Ping, EntityUpdate, ComponentResponse, ComponentQuery];
}

#[dual_message]
pub struct Ping;

#[client_message]
pub struct EntityUpdate {
    pub entities: Vec<RemoteEntity>,
}

#[derive(Default)]
#[client_message]
pub struct ComponentResponse {
    pub components: Vec<Vec<ReflectObject>>,
}

#[derive(Default)]
#[editor_message]
pub struct ComponentQuery {
    pub components: Vec<String>,
    pub entities: Vec<RemoteEntity>,
}
