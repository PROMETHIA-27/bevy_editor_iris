use crate::common::{
    asynchronous::{monitor_remote_thread, open_remote_thread},
    *,
};
use bevy::prelude::*;
use quinn::ClientConfig;

mod interface;
mod systems;

pub use interface::*;

use self::systems::run_client;

pub struct ClientPlugin;

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(open_remote_thread(run_client).exclusive_system())
            .add_system(monitor_remote_thread(run_client).exclusive_system())
            .add_distributor()
            .add_messages::<DefaultMessages>();
    }
}

fn client_config() -> ClientConfig {
    let cert = rustls::Certificate(std::fs::read("certificate.der").unwrap());

    let mut store = rustls::RootCertStore::empty();
    store.add(&cert).unwrap();

    ClientConfig::with_root_certificates(store)
}
