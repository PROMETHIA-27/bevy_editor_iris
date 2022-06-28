use common::deps::bevy::ecs as bevy_ecs;
use common::deps::bevy::prelude::{
    Commands, Component, Entity, Name, Query, Reflect, RemovedComponents, ResMut, Without,
};
use common::deps::bevy::reflect as bevy_reflect;
use common::interface::Interface;

use crate::client::ClientInterfaceExt;

#[derive(Component, Default, Reflect)]
#[component(storage = "SparseSet")]
pub struct TrackedInEditor;

pub fn tag_new_entities(
    mut commands: Commands,
    query: Query<(Entity, Option<&Name>), Without<TrackedInEditor>>,
    interface: ResMut<Interface>,
) {
    if query.is_empty() {
        return;
    }

    for (entity, _) in query.iter() {
        commands.entity(entity).insert(TrackedInEditor);
    }

    _ = interface.send_entity_update(
        query
            .iter()
            .map(|(e, n)| (e, n.map(|name| name.to_string()))),
        false,
    );
}

pub fn tag_deleted_entities(
    removals: RemovedComponents<TrackedInEditor>,
    interface: ResMut<Interface>,
) {
    if removals.iter().next().is_none() {
        return;
    }

    _ = interface.send_entity_update(removals.iter().map(|e| (e, None)), true);
}
