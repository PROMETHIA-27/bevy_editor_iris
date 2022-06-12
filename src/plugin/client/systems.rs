use super::{resources::ClientThread, EditorInterface};
use bevy::prelude::*;

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
            runtime.block_on(communicate_with_editor(connection, client_rx, editor_tx));
        } else {
            println!("Failed to connect to server!");
        }

        println!("Client thread closing!");
    });

    world.insert_non_send_resource(interface);
    world.insert_resource(ClientThread(client_thread));
}

pub fn execute_editor_commands(world: &mut World) {
    let interface: EditorInterface = world.remove_non_send_resource().unwrap();

    for command in interface..iter() {
        if let Some(data) = command(world) {
            data_channel.send(data).unwrap();
        }
    }

    world.insert_non_send_resource(interface);
}

async fn connect_to_editor() -> Result<NewConnection, Error> {
    let endpoint = Endpoint::client(client_addr())?;

    println!("Attempting connection!");

    let connection = endpoint
        .connect_with(client_config(), server_addr(), "localhost")?
        .await?;

    println!("Acquired connection to editor!");

    Ok(connection)
}

async fn communicate_with_editor(
    mut connection: NewConnection,
    comm_tx: Sender<Command>,
    mut data_rx: Receiver<Vec<u8>>,
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
        let (mut send, mut recv) = next_stream.unwrap();

        let mut header = [0; 1];
        let read = recv.read_exact(&mut header).await;
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
