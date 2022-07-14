use crate::dispatcher::{Dispatcher, MessageToConnection, MessageToRoom, MessageToServer};
use crate::packets::{
    BroadcastMessagePacket, HelloRequestPacket, HelloResponsePacket, HelloResponseStatusCode,
    IntoPacket, JoinRoomRequestPacket, JoinRoomResponsePacket, Packet, PacketType,
    ServerNotificationPacket, ServerNotificationType, TestCountUpResponsePacket,
};
use crate::types::{ConnectionID, RoomID};
use async_trait::async_trait;
use bytes::Bytes;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, warn};

#[async_trait]
pub trait Connection {
    fn connection_id(&self) -> ConnectionID;
    async fn send<P>(&mut self, packet: P) -> crate::Result<()>
    where
        P: Send + IntoPacket;
    async fn recv(&mut self) -> crate::Result<Packet>;
}

enum RoomStatus {
    NotJoined,
    Joined { room_id: RoomID },
}

pub(crate) async fn connection_task(
    mut conn: impl Connection,
    mut receiver: mpsc::Receiver<MessageToConnection>,
    dispatcher: Arc<Dispatcher>,
) {
    let connection_id = conn.connection_id();
    debug!("start connection task (connection_id: {})", connection_id);
    let mut handler = ConnectionHandler {
        room_status: RoomStatus::NotJoined,
    };

    loop {
        // TODO: handle shutdown
        tokio::select! {
            Ok(packet) = conn.recv() => {
                handler.handle_packet(&packet, &mut conn, &dispatcher).await;
            }
            Some(msg) = receiver.recv() => {
                handler.handle_message(msg, &mut conn).await;
            }
            else => break
        }
    }
    debug!("drop connection: {}", connection_id);
    dispatcher.drop_connection(&connection_id);
}

struct ConnectionHandler {
    room_status: RoomStatus,
}

impl ConnectionHandler {
    async fn handle_message(&mut self, msg: MessageToConnection, conn: &mut impl Connection) {
        match (&self.room_status, msg) {
            (_, MessageToConnection::Shutdown { reason }) => {
                let packet = ServerNotificationPacket {
                    notification_type: ServerNotificationType::Shutdown,
                };
                if let Err(err) = conn.send(packet).await {
                    warn!("failed to send to client: {:?}", err);
                }
            }
            (RoomStatus::NotJoined, MessageToConnection::JoinResponse { room_id }) => {
                self.room_status = RoomStatus::Joined { room_id };
                let packet = JoinRoomResponsePacket {};
                if let Err(err) = conn.send(packet).await {
                    warn!("failed to send to client: {:?}", err);
                }
            }
            (RoomStatus::Joined { .. }, msg) => match msg {
                MessageToConnection::Broadcast { payload, .. } => {
                    let payload = payload.to_vec();
                    let packet = BroadcastMessagePacket::new(payload);
                    if let Err(err) = conn.send(packet).await {
                        warn!("failed to send to client: {:?}", err);
                    }
                }
                MessageToConnection::TestCountUpResponse { counter } => {
                    let packet = TestCountUpResponsePacket::new(counter);
                    if let Err(err) = conn.send(packet).await {
                        warn!("failed to send to client: {:?}", err);
                    }
                }
                _ => {
                    warn!("unknown message received: {:?}", msg)
                }
            },
            _ => {}
        }
    }

    async fn handle_packet(
        &self,
        packet: &Packet,
        conn: &mut impl Connection,
        dispatcher: &Dispatcher,
    ) {
        match (&self.room_status, &packet.packet_type) {
            (RoomStatus::NotJoined, PacketType::HelloRequest) => {
                let packet = packet.parse_payload().unwrap();
                self.handle_hello(packet, conn).await;
            }
            (RoomStatus::NotJoined, PacketType::JoinRoomRequest) => {
                let packet = packet.parse_payload().unwrap();
                self.handle_join_room(packet, conn, dispatcher).await;
            }
            (RoomStatus::Joined { room_id }, PacketType::BroadcastMessage) => {
                let packet = packet.parse_payload().unwrap();
                self.handle_broadcast(packet, conn, *room_id, dispatcher)
                    .await;
            }
            (RoomStatus::Joined { room_id }, PacketType::TestCountUp) => {
                dispatcher
                    .publish_to_room(
                        room_id,
                        MessageToRoom::TestCountUp {
                            sender: conn.connection_id(),
                        },
                    )
                    .await;
            }
            _ => {
                warn!("unknown packet received: {:?}", packet);
            }
        }
    }

    async fn handle_hello(&self, _: HelloRequestPacket, conn: &mut impl Connection) {
        let resp = HelloResponsePacket {
            status_code: HelloResponseStatusCode::OK,
            message_size: 5,
            message: "hello".as_bytes().to_vec(),
        };
        conn.send(resp).await.unwrap();
    }

    async fn handle_join_room(
        &self,
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

    async fn handle_broadcast(
        &self,
        packet: BroadcastMessagePacket,
        conn: &impl Connection,
        room_id: RoomID,
        dispatcher: &Dispatcher,
    ) {
        dispatcher
            .publish_to_room(
                &room_id,
                MessageToRoom::Broadcast {
                    sender: conn.connection_id(),
                    payload: Bytes::from(packet.payload),
                },
            )
            .await;
    }
}
