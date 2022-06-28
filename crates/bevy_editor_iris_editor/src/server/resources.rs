use std::sync::{Arc, RwLock};

use common::deps::bevy::prelude::{Deref, DerefMut};
use common::deps::bevy::utils::HashMap;

use common::serde::{ReflectObject, RemoteEntity};

#[derive(Clone, Default, Deref, DerefMut)]
// TODO: Maybe use a sorted vec of components and not a map here?
// Maybe a vec as well as a map to jump to vec indices.
pub struct EntityCache(pub Arc<RwLock<HashMap<RemoteEntity, HashMap<String, ReflectObject>>>>);
