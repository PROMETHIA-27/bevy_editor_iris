use super::*;

pub async fn connect_to_editor() -> Result<NewConnection, Box<dyn Error>> {
    let endpoint = Endpoint::client(client_addr())?;

    println!("Attempting connection!");

    let connection = endpoint
        .connect_with(client_config(), server_addr(), "localhost")?
        .await?;

    println!("Acquired connection to editor!");

    Ok(connection)
}

pub async fn communicate_with_editor(
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
        if read.is_ok() {
            if let Ok(EditorMessage::Ping) = header[0].try_into() {
                comm_tx
                    .send(|_| {
                        println!("Received ping!");
                        None
                    })
                    .unwrap();
            }

            // Ping back to confirm
            send.write(&[255]).await.unwrap();
        } else {
            let err = read.unwrap_err();
            println!("Stream closed! Reason: {:?}", err);
            break;
        }
    }
    println!("Ran out of bi streams!");
}
