use super::*;

pub fn start_up_server(world: &mut World) {
    let server_thread = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        runtime.block_on(run_server()).unwrap();
    });
}
