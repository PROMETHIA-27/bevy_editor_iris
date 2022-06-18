use super::{resources::ClientThread, EditorInterface};
use crate::common::*;
use bevy::prelude::*;
use futures_util::StreamExt;
use quinn::{ClientConfig, ConnectError, ConnectionError, Endpoint, NewConnection};
use thiserror::Error;
use tokio::sync::mpsc::{Receiver, Sender};

pub fn open_client_thread(world: &mut World) {
    let (client_tx, client_rx) = tokio::sync::mpsc::channel(128); // See note about buffer size in editor
    let (editor_tx, editor_rx) = tokio::sync::mpsc::channel(128);

    let interface = EditorInterface::new(client_tx, editor_rx);

    let client_thread = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        if let Ok(connection) = runtime.block_on(connect_to_editor()) {
            runtime.block_on(communicate_with_editor(connection, editor_tx, client_rx));
        } else {
            println!("Failed to connect to server!");
        }

        println!("Client thread closing!");
    });

    world.insert_non_send_resource(interface);
    world.insert_resource(ClientThread(client_thread));
}

// pub fn execute_editor_commands(world: &mut World) {
//     let interface: EditorInterface = world.remove_non_send_resource().unwrap();

//     for command in interface..iter() {
//         if let Some(data) = command(world) {
//             data_channel.send(data).unwrap();
//         }
//     }

//     world.insert_non_send_resource(interface);
// }

fn client_config() -> ClientConfig {
    let cert = rustls::Certificate(std::fs::read("certificate.der").unwrap());

    let mut store = rustls::RootCertStore::empty();
    store.add(&cert).unwrap();

    ClientConfig::with_root_certificates(store)
}

#[derive(Debug, Error)]
enum EditorConnectError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    ConnectError(#[from] ConnectError),
    #[error(transparent)]
    ConnectionError(#[from] ConnectionError),
}

async fn connect_to_editor() -> Result<NewConnection, EditorConnectError> {
    let endpoint = Endpoint::client(crate::client_addr())?;

    println!("Attempting connection!");

    let connection = endpoint
        .connect_with(client_config(), crate::server_addr(), "localhost")?
        .await?;

    println!("Acquired connection to editor!");

    Ok(connection)
}

async fn communicate_with_editor(
    mut connection: NewConnection,
    _editor_tx: Sender<Box<dyn EditorMessage>>,
    mut _client_rx: Receiver<Box<dyn ClientMessage>>,
) {
    loop {
        let next_stream = connection.bi_streams.next().await;
        if let None = next_stream {
            println!("Bi streams closed!");
            break;
        }
        let next_stream = next_stream.unwrap();
        if let Err(err) = next_stream {
            println!("Bi stream connection failure: {:?}", err);
            break;
        }
        let (mut _send, mut recv) = next_stream.unwrap();

        let mut header = [0; 1];
        let _read = recv.read_exact(&mut header).await;
        // if read.is_ok() {
        //     if let Ok(EditorMessage::Ping) = header[0].try_into() {
        //         comm_tx
        //             .send(|_| {
        //                 println!("Received ping!");
        //                 None
        //             })
        //             .unwrap();
        //     }

        //     // Ping back to confirm
        //     send.write(&[255]).await.unwrap();
        // } else {
        //     let err = read.unwrap_err();
        //     println!("Stream closed! Reason: {:?}", err);
        //     break;
        // }
    }
    println!("Ran out of bi streams!");
}
