use super::*;

mod r#async;
mod plugin;
mod systems;

pub use plugin::*;
pub use r#async::*;
pub use systems::*;

/// The entry point for the Ouroboros editor application.
///
/// ### Example:
/// ```
/// use bevy_mod_ouroboros::*;
///
/// fn main() {
///     Editor::new().run()
/// }
/// ```
///
/// Note: The `Editor` struct itself cannot be constructed, as its only member is a non-constructible type.
/// The `Editor` type is only a helper to create the editor app ergonomically, and is equivalent to
/// `App::new().add_plugin(EditorPlugin)`.
pub struct Editor(std::convert::Infallible);

impl Editor {
    pub fn new() -> App {
        let mut app = App::new();
        app.add_plugin(EditorPlugin);

        app
    }
}

fn generate_self_signed_cert() -> Result<(rustls::Certificate, rustls::PrivateKey), Box<dyn Error>>
{
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
