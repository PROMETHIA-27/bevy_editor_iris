use crate::common::RemoteEntity;
use bevy::prelude::*;

mod client_interface;

pub use client_interface::*;

#[derive(Default, Deref, DerefMut)]
pub struct EntityCache(pub Vec<RemoteEntity>);
