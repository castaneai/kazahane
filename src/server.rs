use crate::connections::connection_task;
use crate::connections::Connection;
use crate::dispatcher::{Dispatcher, MessageToRoom, MessageToServer};
use crate::room_states::redis::RedisStateStore;
use crate::rooms::room_task;
use crate::transports::websocket;
use crate::RoomID;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::mpsc;

type RoomMap = HashMap<RoomID, ()>;

pub async fn start(listener: &TcpListener, redis: redis::Client) {
    let (sender_to_server, mut receiver) = mpsc::channel(8);
    let dispatcher = Arc::new(Dispatcher::new(sender_to_server));
    let mut rooms = RoomMap::new();
    let redis_conn = redis.get_tokio_connection_manager().await.unwrap();
    loop {
        tokio::select! {
            Ok(conn) = websocket::accept(listener) => {
                let receiver = dispatcher.register_connection(conn.connection_id());
                // TODO: instrument task
                tokio::spawn(connection_task(conn, receiver, dispatcher.clone()));
            }
            Some(msg) = receiver.recv() => {
                handle_message(msg, &mut rooms, dispatcher.clone(), &redis_conn).await;
            }
            else => break
        }
    }
}

async fn handle_message(
    msg: MessageToServer,
    rooms: &mut RoomMap,
    dispatcher: Arc<Dispatcher>,
    redis_conn: &redis::aio::ConnectionManager,
) {
    match msg {
        MessageToServer::Join {
            connection_id,
            room_id,
        } => {
            if !rooms.contains_key(&room_id) {
                let room_receiver = dispatcher.register_room(room_id);
                let room_state = RedisStateStore::new(room_id, redis_conn.clone());
                // TODO: instrument task
                tokio::spawn(room_task(
                    room_id,
                    room_receiver,
                    dispatcher.clone(),
                    room_state,
                ));
            }
            dispatcher
                .publish_to_room(&room_id, MessageToRoom::Join { connection_id })
                .await;
        }
    }
}
