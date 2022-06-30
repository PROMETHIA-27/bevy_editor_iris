use std::{sync::Arc, time::Duration};

use common::deps::bevy::prelude::{App, CoreStage, Plugin};
use common::deps::quinn::{ServerConfig, TransportConfig};
use common::deps::rcgen::{self, RcgenError};
use common::deps::rustls::{Certificate, Error, PrivateKey};
use common::CommonPlugin;

pub use self::resources::EntityCache;

mod resources;
mod systems;

pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(CommonPlugin(systems::run_server))
            .insert_resource(resources::EntityCache::default())
            .add_system_to_stage(CoreStage::PreUpdate, systems::update_entity_cache);
        // .add_system_to_stage(CoreStage::PreUpdate, systems::apply_scene_diff);
    }
}

fn generate_self_signed_cert() -> Result<(Certificate, PrivateKey), RcgenError> {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])?;
    let key = PrivateKey(cert.serialize_private_key_der());
    Ok((Certificate(cert.serialize_der()?), key))
}

fn server_config(cert: Certificate, key: PrivateKey) -> Result<ServerConfig, Error> {
    ServerConfig::with_single_cert(vec![cert], key).map(|mut config| {
        let mut transport = TransportConfig::default();
        transport.keep_alive_interval(Some(Duration::from_secs(5)));

        config.transport = Arc::new(transport);
        config
    })
}
