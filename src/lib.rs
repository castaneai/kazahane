pub mod packets;
pub mod server;
pub mod transports;
mod rooms;
pub mod connections;
mod types;

pub type Result<T> = anyhow::Result<T>;
