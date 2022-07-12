use tokio::sync::mpsc;

pub(crate) type ConnectionID = String;
pub(crate) type RoomID = String;

#[derive(Debug, Clone)]
pub(crate) enum ServerToRoom {
    Join {
        conn_id: ConnectionID,
        sender_to_conn: mpsc::Sender<RoomToConnection>,
    },
    Leave {
        conn_id: ConnectionID,
    },
}

#[derive(Clone, Debug)]
pub(crate) enum RoomToServer {
    JoinResponse {
        sender_to_room: mpsc::Sender<ConnectionToRoom>,
    },
}

#[derive(Clone, Debug)]
pub(crate) enum RoomToConnection {}

#[derive(Clone, Debug)]
pub(crate) enum ConnectionToServer {
    Join { room_id: RoomID },
}

#[derive(Clone, Debug)]
pub(crate) enum ConnectionToRoom {}
