use bevy_egui::EguiPlugin;
use common::deps::bevy::prelude::{App, Plugin};

mod systems;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(EguiPlugin).add_system(systems::ui);
    }
}
