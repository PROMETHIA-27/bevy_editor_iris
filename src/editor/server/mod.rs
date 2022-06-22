use crate::common::*;
use bevy::prelude::*;
use quinn::ServerConfig;
use rcgen::RcgenError;
use systems::*;

mod resources;
mod systems;

pub use resources::{EntityCache, QueryComponentError};

pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugin(CommonPlugin(run_server))
            .insert_resource(resources::EntityCache::default())
            .add_system_to_stage(CoreStage::PreUpdate, systems::update_entity_cache)
            .add_system_set(
                SystemSet::new()
                    .with_run_criteria(run_on_timer(5.0))
                    .with_system(keepalive),
            );
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
    ServerConfig::with_single_cert(vec![cert], key)
}
