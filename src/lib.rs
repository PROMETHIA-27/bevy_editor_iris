#![deny(missing_docs)]
//! The bevy editor called Iris.

pub use editor;
pub use plugin;

/// Contains all the most commonly used imports
pub mod prelude {
    pub use super::editor::{Editor, EditorPlugin};
    pub use super::plugin::IrisClientPlugin;
}
