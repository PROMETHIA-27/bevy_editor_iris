use bevy_editor_iris_common::bevy::prelude::{App, Plugin};
use client::ClientPlugin;
use tabs::TabPlugin;

pub mod client;
pub mod tabs;

pub struct IrisClientPlugin;

impl Plugin for IrisClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ClientPlugin).add_plugin(TabPlugin);
    }
}
