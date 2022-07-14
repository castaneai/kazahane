use crate::types::{ConnectionID, RoomID};
use bytes::Bytes;
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::mpsc;

#[derive(Debug)]
pub(crate) enum MessageToServer {
    Join {
        connection_id: ConnectionID,
        room_id: RoomID,
    },
}

#[derive(Debug)]
pub(crate) enum MessageToRoom {
    Join {
        connection_id: ConnectionID,
    },
    Broadcast {
        sender: ConnectionID,
        payload: Bytes,
    },
}

#[derive(Debug)]
pub(crate) enum MessageToConnection {
    JoinResponse,
    Broadcast {
        room_id: RoomID,
        sender: ConnectionID,
        payload: Bytes,
    },
}

#[derive(Debug)]
pub(crate) struct Dispatcher {
    server_sender: mpsc::Sender<MessageToServer>,
    room_senders: Mutex<HashMap<RoomID, mpsc::Sender<MessageToRoom>>>,
    connection_senders: Mutex<HashMap<ConnectionID, mpsc::Sender<MessageToConnection>>>,
}

impl Dispatcher {
    pub fn new(server_sender: mpsc::Sender<MessageToServer>) -> Self {
        Self {
            server_sender,
            room_senders: Mutex::new(HashMap::new()),
            connection_senders: Mutex::new(HashMap::new()),
        }
    }

    pub async fn publish_to_server(&self, msg: MessageToServer) {
        self.server_sender.send(msg).await.unwrap();
    }

    pub fn register_room(&self, room_id: RoomID) -> mpsc::Receiver<MessageToRoom> {
        let (tx, rx) = mpsc::channel(8);
        self.room_senders.lock().unwrap().insert(room_id, tx);
        rx
    }

    pub fn drop_room(&self, room_id: &RoomID) {
        self.room_senders.lock().unwrap().remove(room_id);
    }

    pub async fn publish_to_room(&self, room_id: &RoomID, msg: MessageToRoom) {
        if let Some(sender) = self.room_sender(room_id) {
            // Ignore the error, because if the receiver was closed, the target room has been deleted.
            let _ = sender.send(msg).await;
        }
    }

    pub fn room_sender(&self, room_id: &RoomID) -> Option<mpsc::Sender<MessageToRoom>> {
        self.room_senders.lock().unwrap().get(room_id).cloned()
    }

    pub fn register_connection(
        &self,
        connection_id: ConnectionID,
    ) -> mpsc::Receiver<MessageToConnection> {
        let (tx, rx) = mpsc::channel(8);
        self.connection_senders
            .lock()
            .unwrap()
            .insert(connection_id, tx);
        rx
    }

    pub fn drop_connection(&self, connection_id: &ConnectionID) {
        self.connection_senders
            .lock()
            .unwrap()
            .remove(connection_id);
    }

    pub async fn publish_to_connection(
        &self,
        connection_id: &ConnectionID,
        msg: MessageToConnection,
    ) {
        if let Some(sender) = self.connection_sender(connection_id) {
            // Ignore the error, because if the receiver was closed, the target connection has been deleted.
            let _ = sender.send(msg).await;
        }
    }

    fn connection_sender(
        &self,
        connection_id: &ConnectionID,
    ) -> Option<mpsc::Sender<MessageToConnection>> {
        self.connection_senders
            .lock()
            .unwrap()
            .get(connection_id)
            .cloned()
    }
}
