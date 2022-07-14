use crate::types::{ConnectionID, RoomID};
use bytes::Bytes;
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum MessageToServer {
    Join {
        connection_id: ConnectionID,
        room_id: RoomID,
    },
    Shutdown {
        reason: ServerShutdownReason,
    },
}

#[derive(Clone, Debug)]
pub enum ServerShutdownReason {
    SigTerm,
}

#[derive(Debug)]
pub enum MessageToRoom {
    Join {
        connection_id: ConnectionID,
    },
    Broadcast {
        sender: ConnectionID,
        payload: Bytes,
    },
    TestCountUp {
        sender: ConnectionID,
    },
}

#[derive(Clone, Debug)]
pub enum MessageToConnection {
    JoinResponse { room_id: RoomID },
    Broadcast { payload: Bytes },
    TestCountUpResponse { counter: usize },
    Shutdown { reason: ServerShutdownReason },
}

#[derive(Debug)]
pub struct Dispatcher {
    server_sender: Mutex<Option<mpsc::Sender<MessageToServer>>>,
    room_senders: Mutex<HashMap<RoomID, mpsc::Sender<MessageToRoom>>>,
    connection_senders: Mutex<HashMap<ConnectionID, mpsc::Sender<MessageToConnection>>>,
}

impl Default for Dispatcher {
    fn default() -> Self {
        Dispatcher::new()
    }
}

impl Dispatcher {
    pub fn new() -> Self {
        Self {
            server_sender: Mutex::new(None),
            room_senders: Mutex::new(HashMap::new()),
            connection_senders: Mutex::new(HashMap::new()),
        }
    }

    pub fn register_server(&self) -> mpsc::Receiver<MessageToServer> {
        let (tx, rx) = mpsc::channel(8);
        let _ = self.server_sender.lock().unwrap().insert(tx);
        rx
    }

    pub async fn publish_to_server(&self, msg: MessageToServer) {
        if let Some(server_sender) = self.server_sender() {
            server_sender.send(msg).await.unwrap();
        }
    }

    fn server_sender(&self) -> Option<mpsc::Sender<MessageToServer>> {
        self.server_sender.lock().unwrap().clone()
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

    pub async fn broadcast_to_connections(&self, msg: MessageToConnection) {
        for sender in self.all_connections() {
            let _ = sender.send(msg.clone()).await;
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

    fn all_connections(&self) -> Vec<mpsc::Sender<MessageToConnection>> {
        self.connection_senders
            .lock()
            .unwrap()
            .values()
            .cloned()
            .collect()
    }
}
