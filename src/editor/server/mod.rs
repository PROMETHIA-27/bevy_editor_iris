use bevy::prelude::*;
use quinn::ServerConfig;
use rcgen::RcgenError;

mod resources;
mod systems;

pub use resources::{ClientInterface, EntityCache, QueryComponentError};

pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_resource(resources::EntityCache::default())
            .add_startup_system(systems::open_server_thread.exclusive_system())
            .add_system_to_stage(CoreStage::PreUpdate, systems::update_entity_cache)
            .add_system(systems::monitor_server_thread.exclusive_system());
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
