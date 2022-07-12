extern crate core;

pub mod connections;
pub mod packets;
mod rooms;
pub mod server;
pub mod transports;
mod types;
mod dispatcher;

pub type Result<T> = anyhow::Result<T>;
pub type ConnectionID = types::ConnectionID;
pub type RoomID = types::RoomID;
