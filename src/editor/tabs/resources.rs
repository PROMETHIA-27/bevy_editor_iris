use bevy::prelude::*;
use std::any::TypeId;

#[derive(Deref, DerefMut)]
pub struct SelectedTab(pub TypeId);
