use super::*;

pub async fn run_server() -> Result<(), Box<dyn Error>> {
    let (cert, key) = generate_self_signed_cert()?;
    std::fs::write("certificate.der", cert.clone())?;
    let server_config = server_config(cert, key)?;

    let (endpoint, mut incoming) = Endpoint::server(server_config, server_addr())?;

    println!("Accepting connections!");

    while let Some(conn) = incoming.next().await {
        println!("Waiting for connection...");

        let new = conn.await?;

        println!("Received a connection!");

        let (mut send, mut recv) = new.connection.open_bi().await?;

        send.write(&[255]).await?;

        let mut buf = [0; 1];
        recv.read_exact(&mut buf).await?;

        let msg: ClientMessage = buf[0].try_into().unwrap();

        assert_eq!(msg, ClientMessage::Ping);

        println!("Received ping response! Closing connection.");
    }

    Ok(())
}
