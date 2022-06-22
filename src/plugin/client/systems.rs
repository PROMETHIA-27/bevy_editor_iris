use super::client_config;
use crate::common::{asynchronous::*, *};
use quinn::Endpoint;
use std::sync::mpsc::{Receiver, Sender};

pub async fn run_client(
    local_rx: Receiver<(StreamId, Box<dyn Message>)>,
    remote_tx: Sender<(StreamId, Box<dyn Message>)>,
    mut stream_counter: StreamCounter,
) -> Result<(), RemoteThreadError> {
    let endpoint = Endpoint::client(crate::client_addr())?;

    println!("Attempting connection!");

    let new = endpoint
        .connect_with(client_config(), crate::server_addr(), "localhost")?
        .await?;

    println!("Acquired connection to editor!");

    process_connection(new, &local_rx, &remote_tx, &mut stream_counter).await?;

    Ok(())
}
