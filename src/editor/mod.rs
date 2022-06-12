use bevy::prelude::*;
use server::ServerPlugin;
use tabs::TabPlugin;
use ui::UiPlugin;

pub mod server;
pub mod tabs;
pub mod ui;

/// The entry point for the Ouroboros editor application.
///
/// ### Example:
/// ```ignore
/// use bevy_mod_ouroboros::*;
///
/// fn main() {
///     Editor::new().run()
/// }
/// ```
///
/// Note: The `Editor` struct itself cannot be constructed, as its only member is a non-constructible type.
/// The `Editor` type is only a helper to create the editor app ergonomically, and is equivalent to
/// `App::new().add_plugin(EditorPlugin)`.
pub struct Editor(std::convert::Infallible);

impl Editor {
    pub fn new() -> App {
        let mut app = App::new();
        app.add_plugin(EditorPlugin);

        app
    }
}

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DefaultPlugins)
            .add_plugin(ServerPlugin)
            .add_plugin(UiPlugin)
            .add_plugin(TabPlugin);
    }
}
