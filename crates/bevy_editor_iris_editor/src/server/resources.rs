use std::sync::{Arc, RwLock};

use bevy_editor_iris_common::bevy::prelude::{Deref, DerefMut};
use bevy_editor_iris_common::bevy::utils::HashMap;

use bevy_editor_iris_common::{ReflectObject, RemoteEntity};

#[derive(Clone, Default, Deref, DerefMut)]
pub struct EntityCache(pub Arc<RwLock<HashMap<RemoteEntity, HashMap<String, ReflectObject>>>>);
