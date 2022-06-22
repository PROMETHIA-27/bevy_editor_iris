use crate::{common::Interface, plugin::client::ClientInterfaceExt};
use bevy::prelude::*;

#[derive(Default, Reflect)]
pub struct TrackedInEditor;

impl Component for TrackedInEditor {
    type Storage = bevy::ecs::component::SparseStorage;
}

pub fn tag_new_entities(
    mut commands: Commands,
    query: Query<Entity, Without<TrackedInEditor>>,
    interface: ResMut<Interface>,
) {
    if query.is_empty() {
        return;
    }

    for entity in query.iter() {
        commands.entity(entity).insert(TrackedInEditor);
    }

    _ = interface.send_entity_update(query.iter(), false);
}

pub fn tag_deleted_entities(
    removals: RemovedComponents<TrackedInEditor>,
    interface: ResMut<Interface>,
) {
    if removals.iter().next().is_none() {
        return;
    }

    _ = interface.send_entity_update(removals.iter(), true);
}
