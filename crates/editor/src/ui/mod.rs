use bevy_egui::EguiPlugin;
use ouroboros_common::bevy::prelude::{Plugin, App};

mod systems;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(EguiPlugin).add_system(systems::ui);
    }
}
