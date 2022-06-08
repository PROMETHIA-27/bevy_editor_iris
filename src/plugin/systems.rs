use super::*;

pub fn open_client_thread(world: &mut World) {
    let (comm_tx, comm_rx) = std::sync::mpsc::channel();
    let (data_tx, data_rx) = std::sync::mpsc::channel();

    let client_thread = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        if let Ok(connection) = runtime.block_on(connect_to_editor()) {
            runtime.block_on(communicate_with_editor(connection, comm_tx, data_rx));
        } else {
            println!("Failed to connect to server!");
        }

        println!("Client thread closing!");
    });

    world.insert_non_send_resource(EditorCommandChannel(comm_rx));
    world.insert_non_send_resource(EditorDataChannel(data_tx));
    world.insert_resource(ClientThread(client_thread));
}

pub fn execute_editor_commands(world: &mut World) {
    let comm_channel: EditorCommandChannel = world.remove_non_send_resource().unwrap();
    let data_channel: EditorDataChannel = world.remove_non_send_resource().unwrap();

    for command in comm_channel.iter() {
        if let Some(data) = command(world) {
            data_channel.send(data).unwrap();
        }
    }

    world.insert_non_send_resource(comm_channel);
    world.insert_non_send_resource(data_channel);
}
