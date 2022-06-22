use asynchronous::{monitor_remote_thread, open_remote_thread, RemoteThreadError};
use bevy::prelude::*;
use futures_util::Future;
use std::sync::{
    atomic::AtomicUsize,
    mpsc::{Receiver, Sender},
    Arc,
};

pub mod asynchronous;
mod interface;
pub mod message;
mod serde;
mod systems;

pub use self::serde::*;
pub use interface::*;
pub use systems::*;

pub use message::{
    AppRegisterMsgExt, DefaultMessages, Message, MessageDistributor, MessageReceived,
    ReflectMessage, ReflectMessageFromReflect, RegisterMessage, SendMessage,
};

pub struct CommonPlugin<
    Run: 'static
        + Fn(
            Receiver<(StreamId, Box<dyn Message>)>,
            Sender<(StreamId, Box<dyn Message>)>,
            Arc<AtomicUsize>,
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
                Arc<AtomicUsize>,
            ) -> F
            + Send
            + Sync
            + Copy,
        F: 'static + Future<Output = Result<(), RemoteThreadError>>,
    > Plugin for CommonPlugin<Run, F>
{
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_startup_system(open_remote_thread(self.0).exclusive_system())
            .add_system_set(
                SystemSet::new()
                    .with_run_criteria(run_on_timer(1.0))
                    .with_system(monitor_remote_thread(self.0).exclusive_system()),
            )
            .add_distributor()
            .add_messages::<DefaultMessages>();
    }
}
