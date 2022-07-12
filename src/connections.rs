use crate::packets::{
    HelloResponsePacket, HelloResponseStatusCode, JoinRoomRequestPacket, KazahanePacket, PacketType,
};
use crate::types::{ConnectionID, ConnectionToServer, RoomToConnection};
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::debug;

pub(crate) async fn connection_task(
    mut conn: impl Connection,
    sender_to_server: mpsc::Sender<ConnectionToServer>,
) {
    loop {
        // TODO: handle shutdown
        while let Ok(msg) = conn.recv().await {
            match msg.packet_type {
                PacketType::HelloRequest => {
                    let resp = KazahanePacket::new(
                        PacketType::HelloResponse,
                        HelloResponsePacket {
                            status_code: HelloResponseStatusCode::OK,
                            message_size: 5,
                            message: "hello".as_bytes().to_vec(),
                        },
                    )
                    .unwrap();
                    conn.send(&resp).await.unwrap();
                }
                PacketType::JoinRoomRequest => {
                    let req = msg.parse_payload::<JoinRoomRequestPacket>().unwrap();
                    debug!("join request: {:?}", req);
                }
                _ => {
                    debug!("received: {:?}", msg);
                }
            }
        }
    }
}

#[async_trait]
pub trait Connection {
    async fn send(&mut self, packet: &KazahanePacket) -> crate::Result<()>;
    async fn recv(&mut self) -> crate::Result<KazahanePacket>;
}

#[derive(Debug)]
pub(crate) struct ConnectionMap {
    connections: HashMap<ConnectionID, mpsc::Sender<RoomToConnection>>,
}

impl ConnectionMap {
    pub fn insert(&mut self, conn_id: ConnectionID, tx: mpsc::Sender<RoomToConnection>) {
        self.connections.insert(conn_id, tx);
    }

    pub fn remove(&mut self, conn_id: ConnectionID) {
        self.connections.remove(conn_id.as_str());
    }

    pub async fn broadcast(&self, msg: RoomToConnection) {
        for tx in self.connections.values() {
            // TODO: parallel
            // TODO: error handling
            tx.send(msg.clone()).await.unwrap();
        }
    }
}
