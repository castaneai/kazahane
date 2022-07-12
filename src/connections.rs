use crate::dispatcher::{Dispatcher, MessageToConnection, MessageToRoom, MessageToServer};
use crate::packets::{
    BroadcastMessagePacket, HelloRequestPacket, HelloResponsePacket, HelloResponseStatusCode,
    JoinRoomRequestPacket, JoinRoomResponsePacket, KazahanePacket, PacketType,
};
use crate::types::{ConnectionID, RoomID};
use async_trait::async_trait;
use bytes::Bytes;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, warn};

pub(crate) async fn connection_task(
    mut conn: impl Connection,
    mut receiver: mpsc::Receiver<MessageToConnection>,
    dispatcher: Arc<Dispatcher>,
) {
    let connection_id = conn.connection_id();
    defer! {
        debug!("drop connection: {}", connection_id);
        dispatcher.drop_connection(&connection_id);
    }
    debug!("start connection task (connection_id: {})", connection_id);

    loop {
        // TODO: handle shutdown
        tokio::select! {
            Ok(packet) = conn.recv() => {
                handle_packet_from_conn(&packet, &mut conn, &dispatcher).await;
            }
            Some(msg) = receiver.recv() => {
                handle_message(msg, &mut conn).await;
            }
        }
    }
}

async fn handle_message(msg: MessageToConnection, conn: &mut impl Connection) {
    match msg {
        MessageToConnection::Broadcast {
            room_id,
            sender,
            payload,
        } => {
            let payload = payload.to_vec();
            let packet = KazahanePacket::new(
                PacketType::BroadcastMessage,
                BroadcastMessagePacket::new(sender.into_bytes(), room_id.into_bytes(), payload),
            )
            .unwrap();
            conn.send(&packet).await.unwrap();
        }
        MessageToConnection::JoinResponse => {
            let packet =
                KazahanePacket::new(PacketType::JoinRoomResponse, JoinRoomResponsePacket {})
                    .unwrap();
            conn.send(&packet).await.unwrap();
        }
    }
}

async fn handle_packet_from_conn(
    packet: &KazahanePacket,
    conn: &mut impl Connection,
    dispatcher: &Arc<Dispatcher>,
) {
    debug!("received from conn: {:?}", packet);
    match packet.packet_type {
        PacketType::HelloRequest => {
            let packet = packet.parse_payload().unwrap();
            handle_hello(packet, conn).await;
        }
        PacketType::JoinRoomRequest => {
            let packet = packet.parse_payload().unwrap();
            handle_join_room(packet, conn, dispatcher).await;
        }
        PacketType::BroadcastMessage => {
            let packet = packet.parse_payload().unwrap();
            handle_broadcast(packet, dispatcher).await;
        }
        _ => {
            warn!("unknown message received: {:?}", packet);
        }
    }
}

async fn handle_hello(_: HelloRequestPacket, conn: &mut impl Connection) {
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

async fn handle_join_room(
    req: JoinRoomRequestPacket,
    conn: &impl Connection,
    dispatcher: &Dispatcher,
) {
    dispatcher
        .publish_to_server(MessageToServer::Join {
            connection_id: conn.connection_id(),
            room_id: RoomID::from_bytes(req.room_id),
        })
        .await;
}

async fn handle_broadcast(packet: BroadcastMessagePacket, dispatcher: &Dispatcher) {
    let room_id = RoomID::from_bytes(packet.room_id);
    let sender_connection_id = ConnectionID::from_bytes(packet.sender);
    dispatcher
        .publish_to_room(
            &room_id,
            MessageToRoom::Broadcast {
                sender: sender_connection_id,
                payload: Bytes::from(packet.payload),
            },
        )
        .await;
}

#[async_trait]
pub trait Connection {
    fn connection_id(&self) -> ConnectionID;
    async fn send(&mut self, packet: &KazahanePacket) -> crate::Result<()>;
    async fn recv(&mut self) -> crate::Result<KazahanePacket>;
}
