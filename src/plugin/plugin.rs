use super::*;

pub struct OuroborosClientPlugin;

impl Plugin for OuroborosClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(open_client_thread.exclusive_system())
            .add_system(execute_editor_commands.exclusive_system());
    }
}
