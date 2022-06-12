pub mod common;
pub mod editor;
pub mod plugin;

pub mod prelude {
    pub use super::editor::{Editor, EditorPlugin};
    pub use super::plugin::OuroborosClientPlugin;
}

// TODO: These won't be necessary forever
fn server_addr() -> std::net::SocketAddr {
    "127.0.0.1:5001".parse().unwrap()
}

fn client_addr() -> std::net::SocketAddr {
    "127.0.0.1:5000".parse().unwrap()
}
