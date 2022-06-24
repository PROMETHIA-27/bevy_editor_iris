use std::sync::{Arc, RwLock};

use crate::common::RemoteEntity;
use bevy::{prelude::*, utils::HashMap};

#[derive(Clone, Default, Deref, DerefMut)]
pub struct EntityCache(pub Arc<RwLock<HashMap<RemoteEntity, Option<String>>>>);
