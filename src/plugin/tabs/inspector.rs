use crate::plugin::client::EditorInterface;
use bevy::prelude::*;

#[derive(Default, Reflect)]
pub struct TrackedInEditor;

impl Component for TrackedInEditor {
    type Storage = bevy::ecs::component::SparseStorage;
}

pub fn tag_new_entities(
    mut commands: Commands,
    query: Query<Entity, Without<TrackedInEditor>>,
    mut interface: NonSendMut<EditorInterface>,
) {
    for entity in query.iter() {
        commands.entity(entity).insert(TrackedInEditor);
    }

    _ = interface.send_entity_update(query.iter().collect());
}

pub fn tag_deleted_entities(
    removals: RemovedComponents<TrackedInEditor>,
    mut interface: NonSendMut<EditorInterface>,
) {
    _ = interface.send_entity_update(removals.iter().collect());
}
