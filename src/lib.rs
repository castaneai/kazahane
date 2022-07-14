extern crate core;
#[macro_use(defer)]
extern crate scopeguard;

pub mod connections;
mod dispatcher;
pub mod packets;
pub mod pubsub;
mod rooms;
pub mod server;
pub mod states;
pub mod transports;
mod types;

pub type Result<T> = anyhow::Result<T>;
pub type ConnectionID = types::ConnectionID;
pub type RoomID = types::RoomID;
