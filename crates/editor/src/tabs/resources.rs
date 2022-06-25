use std::any::TypeId;

use ouroboros_common::bevy::prelude::{Deref, DerefMut};

// TODO: Make this optional and add a "no selected tab" screen
#[derive(Deref, DerefMut)]
pub struct SelectedTab(pub TypeId);
