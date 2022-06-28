use client::ClientPlugin;
use common::deps::bevy::prelude::{App, Plugin};
use tabs::TabPlugin;

pub mod client;
pub mod tabs;

pub mod deps {
    pub use common;
}

pub struct IrisClientPlugin;

impl Plugin for IrisClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ClientPlugin).add_plugin(TabPlugin);
    }
}
