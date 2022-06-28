use std::any::TypeId;

use common::deps::bevy::prelude::{Deref, DerefMut};

// TODO: Make this optional and add a "no selected tab" screen
#[derive(Deref, DerefMut)]
pub struct SelectedTab(pub TypeId);
