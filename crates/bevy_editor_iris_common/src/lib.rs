#![deny(missing_docs)]
//! This crate contains common logic for the plugin/client and editor crates of bevy_editor_iris.
//!
//! This crate mainly provides the networking infrastructure of the editor.
//! At a high level:
//! - Messages are represented as reflectable types which can be serialized and deserialized automatically at both ends
//! - A new thread is spun up, the remote thread. The remote thread runs a tokio runtime which drives quinn, the QUIC protocol library.
//! - Messages are sent between the remote thread and the local threads (all other threads) via channels.
//! - Sending a message without a StreamId creates a new "transaction", represented as a stream.
//! - When a message is received, the corresponding StreamId is kept with it.
//! - Sending a message with a StreamId sends it to that transaction.
//! - Messages that are received are distributed via bevy's event system.

use std::borrow::Cow;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

use asynchronous::{OpeningReceiver, OpeningSender};
use bevy::math::Vec3A;
use bevy::prelude::{IntoExclusiveSystem, Plugin, SystemSet};
use futures_lite::Future;

use self::asynchronous::RemoteThreadError;
// use self::message::distributor::{self, AppRegisterMsgExt};
use self::message::messages::DefaultMessages;
use self::message::Message;

/// Contains asynchronous logic using tokio which powers the remote thread
pub mod asynchronous;
/// Contains logic binding the local and remote threads together
pub mod interface;
/// Contains utility macros
pub mod macros;
/// Contains message infrastructure and some built-in message definitions
pub mod message;
/// Contains logic related to serializing and deserializing reflected types and messages
pub mod serde;
/// Contains local-thread logic which both the editor and client depend on
pub mod systems;

/// Contains all the most commonly used imports for easy usage.
pub mod prelude {
    pub use super::interface::{Interface, InterfaceError};
    // pub use super::message::distributor::AppRegisterMsgExt;
    pub use super::message::{IntoAny, IntoReflect, Message};
    pub use super::serde::{ReflectObject, RemoteEntity};
}

/// Contains re-exports of dependencies
pub mod deps {
    pub use bevy;
    pub use futures_lite;
    pub use quinn;
    pub use rcgen;
    pub use rustls;
    pub use tokio;
}

/// Handles common logic for both the editor and client components of the iris editor.,
/// including opening the remote thread and registering messages.
pub struct CommonPlugin<
    Run: 'static
        + Fn(
            OpeningSender,
            OpeningReceiver,
            // StreamCounter,
        ) -> F
        + Send
        + Sync
        + Copy,
    F: 'static + Future<Output = Result<(), RemoteThreadError>>,
>(pub Run);

impl<
        Run: 'static
            + Fn(
                OpeningSender,
                OpeningReceiver,
                // StreamCounter,
            ) -> F
            + Send
            + Sync
            + Copy,
        F: 'static + Future<Output = Result<(), RemoteThreadError>>,
    > Plugin for CommonPlugin<Run, F>
{
    fn build(&self, app: &mut bevy::prelude::App) {
        // app.insert_resource(StreamCounter::default())
        // .add_startup_system(asynchronous::open_remote_thread(self.0).exclusive_system())
        // .add_system_set(
        //     SystemSet::new()
        //         .with_run_criteria(systems::run_on_timer(Duration::from_secs(1)))
        //         .with_system(systems::monitor_remote_thread(self.0).exclusive_system()),
        // )
        // app.add_distributor()
        // .add_messages::<DefaultMessages>()
        // .add_system(distributor::distribute_messages.exclusive_system())
        // .add_system(distributor::collect_messages.exclusive_system())
        app.register_type::<Cow<'static, str>>()
            .register_type::<Vec3A>();
    }
}

// TODO: These won't be necessary forever
/// The address of the server
pub fn server_addr() -> std::net::SocketAddr {
    "127.0.0.1:5001".parse().unwrap()
}

/// The address of the client
pub fn client_addr() -> std::net::SocketAddr {
    "127.0.0.1:5000".parse().unwrap()
}

#[test]
fn playground() {
    use tokio::select;
    use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

    let (remote_tx, local_rx) = unbounded_channel();
    let (local_tx, remote_rx) = unbounded_channel();

    let remote_thread = std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let pending_messages = futures::stream::FuturesUnordered::new();
                let received_messages = futures::stream::FuturesUnordered::new();

                loop {
                    select! {
                        (trans_tx, trans_rx) = remote_rx.recv() => {
                            // create_connection()
                        }
                        msg = pending_messages.next() => {

                        }
                    }
                }
            });

        async fn create_connection<G: Future>(
            trans_tx: UnboundedSender<impl Future>,
            trans_rx: UnboundedReceiver<G>,
            connection: quinn::NewConnection,
        ) {
            let streams = connection.connection.open_bi().await?;

            received_messages.push(async move {
                let (send, recv) = streams;

                let msg = todo!(); // read message from recv

                (send, recv, msg)
            })
        }
    });

    let (remote_trans_tx, local_trans_rx) = unbounded_channel();
    let (local_trans_tx, remote_trans_rx) = unbounded_channel();
    local_tx.send((remote_trans_tx, remote_trans_rx));
}
