use bevy::prelude::*;

mod inspector;

pub use inspector::*;

pub struct TabPlugin;

impl Plugin for TabPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(inspector::tag_deleted_entities)
            .add_system(inspector::tag_new_entities);
    }
}
