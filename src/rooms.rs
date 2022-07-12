use crate::dispatcher::{Dispatcher, MessageToConnection, MessageToRoom};
use crate::types::{ConnectionID, RoomID};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::debug;

pub(crate) async fn room_task(
    room_id: RoomID,
    mut receiver: mpsc::Receiver<MessageToRoom>,
    dispatcher: Arc<Dispatcher>,
) {
    debug!("start room task (room_id: {})", room_id);
    let mut room = Room {
        room_id,
        connections: HashMap::new(),
    };
    loop {
        // TODO: shutdown room
        tokio::select! {
            Some(msg) = receiver.recv() => {
                room.handle_message(msg, &dispatcher).await;
            }
        }
    }
}

#[derive(Debug)]
struct Room {
    room_id: RoomID,
    connections: HashMap<ConnectionID, ()>,
}

impl Room {
    async fn handle_message(&mut self, msg: MessageToRoom, dispatcher: &Dispatcher) {
        match msg {
            MessageToRoom::Join { connection_id } => {
                self.connections.insert(connection_id, ());
                debug!("[{}] client joined: {}", self.room_id, connection_id);
                dispatcher
                    .publish_to_connection(&connection_id, MessageToConnection::JoinResponse)
                    .await;
            }
            MessageToRoom::Broadcast { sender, payload } => {
                for connection_id in self.connections.keys() {
                    dispatcher
                        .publish_to_connection(
                            connection_id,
                            MessageToConnection::Broadcast {
                                room_id: self.room_id,
                                sender,
                                payload: payload.clone(),
                            },
                        )
                        .await;
                }
            }
        }
    }
}
