use std::sync::mpsc::{Receiver, Sender};

use bevy::prelude::{IntoExclusiveSystem, Plugin, SystemSet};
use futures_util::Future;

use self::asynchronous::RemoteThreadError;
use self::message::distributor;

pub use bevy;
pub use futures_util;
pub use quinn;
pub use rcgen;
pub use rustls;
pub use tokio;

pub use self::interface::{Interface, InterfaceError, StreamCounter, StreamId};
pub use self::message::{
    AppRegisterMsgExt, DefaultMessages, Message, MessageReceived, MessageWriter, ReflectMessage,
    ReflectMessageFromReflect, RegisterMessage,
};
pub use self::serde::{ReflectObject, RemoteEntity};

pub mod asynchronous;
mod interface;
pub mod message;
pub mod serde;
pub mod systems;

pub struct CommonPlugin<
    Run: 'static
        + Fn(
            Receiver<(StreamId, Box<dyn Message>)>,
            Sender<(StreamId, Box<dyn Message>)>,
            StreamCounter,
        ) -> F
        + Send
        + Sync
        + Copy,
    F: 'static + Future<Output = Result<(), RemoteThreadError>>,
>(pub Run);

impl<
        Run: 'static
            + Fn(
                Receiver<(StreamId, Box<dyn Message>)>,
                Sender<(StreamId, Box<dyn Message>)>,
                StreamCounter,
            ) -> F
            + Send
            + Sync
            + Copy,
        F: 'static + Future<Output = Result<(), RemoteThreadError>>,
    > Plugin for CommonPlugin<Run, F>
{
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_resource(StreamCounter::default())
            .add_startup_system(asynchronous::open_remote_thread(self.0).exclusive_system())
            .add_system_set(
                SystemSet::new()
                    .with_run_criteria(systems::run_on_timer(1.0))
                    .with_system(systems::monitor_remote_thread(self.0).exclusive_system()),
            )
            .add_distributor()
            .add_messages::<DefaultMessages>()
            .add_system(distributor::distribute_messages.exclusive_system())
            .add_system(distributor::collect_messages.exclusive_system());
    }
}

// TODO: These won't be necessary forever
pub fn server_addr() -> std::net::SocketAddr {
    "127.0.0.1:5001".parse().unwrap()
}

pub fn client_addr() -> std::net::SocketAddr {
    "127.0.0.1:5000".parse().unwrap()
}
