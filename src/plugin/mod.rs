use super::*;

mod r#async;
mod plugin;
mod resources;
mod systems;

pub use plugin::*;
pub use r#async::*;
pub use resources::*;
pub use systems::*;

type Command = fn(&mut World) -> Option<Vec<u8>>;

fn client_config() -> ClientConfig {
    let cert = rustls::Certificate(std::fs::read("certificate.der").unwrap());

    let mut store = rustls::RootCertStore::empty();
    store.add(&cert).unwrap();

    ClientConfig::with_root_certificates(store)
}
