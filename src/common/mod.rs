use asynchronous::{open_remote_thread, RemoteThreadError};
use bevy::prelude::*;
use futures_util::Future;
use message::{collect_messages, distribute_messages};
use std::sync::mpsc::{Receiver, Sender};

pub mod asynchronous;
mod interface;
pub mod message;
mod serde;
mod systems;

pub use self::serde::*;
pub use interface::*;
pub use systems::*;

pub use message::{
    AppRegisterMsgExt, DefaultMessages, Message, MessageReceived, MessageWriter, ReflectMessage,
    ReflectMessageFromReflect, RegisterMessage,
};

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
            .add_startup_system(open_remote_thread(self.0).exclusive_system())
            .add_system_set(
                SystemSet::new()
                    .with_run_criteria(run_on_timer(1.0))
                    .with_system(monitor_remote_thread(self.0).exclusive_system()),
            )
            .add_distributor()
            .add_messages::<DefaultMessages>()
            .add_system(distribute_messages.exclusive_system())
            .add_system(collect_messages.exclusive_system());
    }
}
