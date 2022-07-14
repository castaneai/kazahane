extern crate core;

pub mod connections;
mod dispatcher;
pub mod packets;
pub mod pubsub;
pub mod room_states;
mod rooms;
pub mod server;
pub mod transports;
mod types;

pub type Result<T> = anyhow::Result<T>;
pub type ConnectionID = types::ConnectionID;
pub type RoomID = types::RoomID;
