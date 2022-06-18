use crate::common::RemoteEntity;
use bevy::prelude::*;
use std::thread::JoinHandle;

mod client_interface;

pub use client_interface::*;

#[derive(Deref, DerefMut)]
pub struct ServerThread(pub JoinHandle<Result<(), super::systems::RunServerError>>);

#[derive(Default, Deref, DerefMut)]
pub struct EntityCache(pub Vec<RemoteEntity>);
