use bevy::prelude::*;

mod resources;
mod systems;

pub use resources::EditorInterface;

pub struct ClientPlugin;

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(systems::open_client_thread.exclusive_system());
    }
}
