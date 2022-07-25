use std::sync::mpsc::channel;

use common::deps::bevy::prelude::{App, Plugin};
pub use resources::InspectorCache;
pub use tab::InspectorTab;

mod messages;
mod resources;
mod systems;
mod tab;

struct InspectorTabPlugin;

impl Plugin for InspectorTabPlugin {
    fn build(&self, app: &mut App) {}
}
