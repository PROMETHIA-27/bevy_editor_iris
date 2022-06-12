use bevy::prelude::*;
use client::ClientPlugin;
use quinn::ClientConfig;
use tabs::TabPlugin;

pub mod client;
pub mod tabs;

fn client_config() -> ClientConfig {
    let cert = rustls::Certificate(std::fs::read("certificate.der").unwrap());

    let mut store = rustls::RootCertStore::empty();
    store.add(&cert).unwrap();

    ClientConfig::with_root_certificates(store)
}

pub struct OuroborosClientPlugin;

impl Plugin for OuroborosClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ClientPlugin).add_plugin(TabPlugin);
    }
}
