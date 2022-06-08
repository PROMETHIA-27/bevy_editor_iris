use super::*;

#[derive(Deref, DerefMut)]
pub struct EditorCommandChannel(pub Receiver<Command>);

#[derive(Deref, DerefMut)]
pub struct EditorDataChannel(pub Sender<Vec<u8>>);

#[derive(Deref, DerefMut)]
pub struct ClientThread(pub JoinHandle<()>);
