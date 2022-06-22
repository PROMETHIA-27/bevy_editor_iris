use crate::common::*;
use bevy::prelude::*;
use quinn::ClientConfig;

mod interface;
mod systems;

pub use interface::*;

use self::systems::run_client;

pub struct ClientPlugin;

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(CommonPlugin(run_client));
    }
}

fn client_config() -> ClientConfig {
    let cert = rustls::Certificate(std::fs::read("certificate.der").unwrap());

    let mut store = rustls::RootCertStore::empty();
    store.add(&cert).unwrap();

    ClientConfig::with_root_certificates(store)
}
