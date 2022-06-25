pub use ouroboros_common as common;
pub use ouroboros_editor as editor;
pub use ouroboros_plugin as plugin;

pub mod prelude {
    pub use super::editor::{Editor, EditorPlugin};
    pub use super::plugin::OuroborosClientPlugin;
}
