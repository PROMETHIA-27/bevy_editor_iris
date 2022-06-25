use std::sync::mpsc::{Receiver, Sender};

use ouroboros_common::asynchronous::{self, RemoteThreadError};
use ouroboros_common::quinn::Endpoint;
use ouroboros_common::{Message, StreamCounter, StreamId};

use super::client_config;

pub async fn run_client(
    local_rx: Receiver<(StreamId, Box<dyn Message>)>,
    remote_tx: Sender<(StreamId, Box<dyn Message>)>,
    mut stream_counter: StreamCounter,
) -> Result<(), RemoteThreadError> {
    let endpoint = Endpoint::client(ouroboros_common::client_addr())?;

    println!("Attempting connection!");

    let new = endpoint
        .connect_with(
            client_config(),
            ouroboros_common::server_addr(),
            "localhost",
        )?
        .await?;

    println!("Acquired connection to editor!");

    asynchronous::process_connection(new, &local_rx, &remote_tx, &mut stream_counter).await?;

    Ok(())
}
