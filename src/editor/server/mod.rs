use std::{sync::Arc, time::Duration};

use crate::common::*;
use bevy::prelude::*;
use quinn::{ServerConfig, TransportConfig};
use rcgen::RcgenError;
use systems::*;

mod resources;
mod systems;

pub use resources::EntityCache;

pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugin(CommonPlugin(run_server))
            .insert_resource(resources::EntityCache::default())
            .add_system_to_stage(CoreStage::PreUpdate, systems::update_entity_cache);
    }
}

fn generate_self_signed_cert() -> Result<(rustls::Certificate, rustls::PrivateKey), RcgenError> {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])?;
    let key = rustls::PrivateKey(cert.serialize_private_key_der());
    Ok((rustls::Certificate(cert.serialize_der()?), key))
}

fn server_config(
    cert: rustls::Certificate,
    key: rustls::PrivateKey,
) -> Result<ServerConfig, rustls::Error> {
    ServerConfig::with_single_cert(vec![cert], key).map(|mut config| {
        let mut transport = TransportConfig::default();
        transport.keep_alive_interval(Some(Duration::from_secs(5)));

        config.transport = Arc::new(transport);
        config
    })
}
