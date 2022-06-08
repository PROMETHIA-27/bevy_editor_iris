use bevy::prelude::*;
use futures_util::StreamExt;
use quinn::*;
use std::{
    error::Error,
    sync::mpsc::{Receiver, Sender},
    thread::JoinHandle,
};

mod editor;
mod plugin;

pub use editor::{Editor, EditorPlugin};
pub use plugin::OuroborosClientPlugin;

fn server_addr() -> std::net::SocketAddr {
    "127.0.0.1:5001".parse().unwrap()
}

fn client_addr() -> std::net::SocketAddr {
    "127.0.0.1:5000".parse().unwrap()
}

#[repr(u8)]
#[non_exhaustive]
#[derive(Clone, Copy, Debug, num_enum::TryFromPrimitive, PartialEq, Eq)]
enum EditorMessage {
    Ping = 255,
}

#[repr(u8)]
#[non_exhaustive]
#[derive(Clone, Copy, Debug, num_enum::TryFromPrimitive, PartialEq, Eq)]
enum ClientMessage {
    Ping = 255,
}
