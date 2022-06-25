pub use bevy_editor_iris_common as common;
pub use bevy_editor_iris_editor as editor;
pub use bevy_editor_iris_plugin as plugin;

pub mod prelude {
    pub use super::editor::{Editor, EditorPlugin};
    pub use super::plugin::IrisClientPlugin;
}
