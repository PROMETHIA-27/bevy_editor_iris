use bevy::prelude::*;
use client::ClientPlugin;
use tabs::TabPlugin;

pub mod client;
pub mod tabs;

pub struct OuroborosClientPlugin;

impl Plugin for OuroborosClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ClientPlugin).add_plugin(TabPlugin);
    }
}
