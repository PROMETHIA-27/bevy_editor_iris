use bevy::prelude::*;
use std::any::TypeId;

// TODO: Make this optional and add a "no selected tab" screen
#[derive(Deref, DerefMut)]
pub struct SelectedTab(pub TypeId);
