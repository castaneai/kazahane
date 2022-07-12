use crate::connections::ConnectionMap;
use crate::types::{RoomID, RoomToConnection, ServerToRoom};
use std::collections::HashMap;
use tokio::sync::mpsc;

#[derive(Debug)]
pub(crate) struct RoomMap {
    rooms: HashMap<RoomID, mpsc::Sender<ServerToRoom>>,
}

impl RoomMap {
    pub fn new() -> Self {
        Self {
            rooms: HashMap::new(),
        }
    }

    async fn broadcast(&self, msg: ServerToRoom) {
        for tx in self.rooms.values() {
            // TODO: error handling
            tx.send(msg.clone()).await.unwrap();
        }
    }
}

#[derive(Debug)]
struct Room {
    room_id: RoomID,
    connections: ConnectionMap,
}

impl Room {
    async fn handle_master_to_room(&mut self, msg: ServerToRoom) {
        match msg {
            ServerToRoom::Join {
                conn_id,
                sender_to_conn,
            } => {
                self.connections.insert(conn_id, sender_to_conn);
            }
            ServerToRoom::Leave { conn_id } => {
                self.connections.remove(conn_id);
            }
        }
    }
}
