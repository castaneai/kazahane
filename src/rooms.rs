use crate::dispatcher::{Dispatcher, MessageToConnection, MessageToRoom};
use crate::pubsub::{PubSub, PubSubMessage, PubSubTopic};
use crate::room_states::{RoomStateStore, StateData};
use crate::types::{ConnectionID, RoomID, ServerID};
use bytes::Bytes;
use core::convert::TryInto;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error};

pub(crate) async fn room_task(
    server_id: ServerID,
    room_id: RoomID,
    mut receiver: mpsc::Receiver<MessageToRoom>,
    dispatcher: Arc<Dispatcher>,
    mut state: impl RoomStateStore,
    mut pubsub: impl PubSub,
) {
    debug!("start room task (room_id: {})", room_id);
    let mut room = Room {
        server_id,
        room_id,
        connections: HashMap::new(),
    };
    let topic = format!("{}", room_id);
    let mut sub = pubsub.subscribe(topic).await.unwrap();
    loop {
        // TODO: shutdown room
        tokio::select! {
            Some(msg) = receiver.recv() => {
                room.handle_message(msg, &dispatcher, &mut pubsub, &mut state).await;
            }
            Ok(Some(msg)) = sub.next_message() => {
                match PubSubMessage::from_bytes(msg) {
                    Ok(msg) => {
                        room.handle_pubsub_message(&msg, &dispatcher).await;
                    }
                    Err(err) => {
                        error!("failed to parse pubsub message: {:?}", err)
                    }
                }
            }
            else => break
        }
    }
    debug!("drop room: {}", room_id);
    dispatcher.drop_room(&room_id);
}

#[derive(Debug)]
struct Room {
    server_id: ServerID,
    room_id: RoomID,
    connections: HashMap<ConnectionID, ()>,
}

impl Room {
    async fn handle_message(
        &mut self,
        msg: MessageToRoom,
        dispatcher: &Dispatcher,
        pubsub: &mut impl PubSub,
        state: &mut impl RoomStateStore,
    ) {
        match msg {
            MessageToRoom::Join { connection_id } => {
                self.connections.insert(connection_id, ());
                debug!("[{}] client joined: {}", self.room_id, connection_id);
                dispatcher
                    .publish_to_connection(
                        &connection_id,
                        MessageToConnection::JoinResponse {
                            room_id: self.room_id,
                        },
                    )
                    .await;
            }
            MessageToRoom::Broadcast {
                payload, sender, ..
            } => {
                self.broadcast(sender, payload.clone(), dispatcher).await;

                // Broadcast to all clients on other servers.
                let msg = PubSubMessage::Broadcast {
                    sender_server: self.server_id.into_bytes(),
                    sender: sender.into_bytes(),
                    payload: payload.to_vec(),
                };
                if let Err(err) = pubsub.publish(self.topic(), msg).await {
                    error!("failed to publish broadcast message: {:?}", err);
                }
            }
            MessageToRoom::TestCountUp { sender } => {
                let to_usize = |d: StateData| usize::from_le_bytes(d.try_into().unwrap());
                let mut counter = state
                    .get_state("counter".into())
                    .await
                    .unwrap()
                    .map_or_else(|| 0, to_usize);
                debug!("[{}] counter {} -> {}", self.room_id, counter, counter + 1);
                counter += 1;
                let write_data = counter.to_le_bytes().to_vec();
                state
                    .save_state("counter".into(), write_data)
                    .await
                    .unwrap();
                dispatcher
                    .publish_to_connection(
                        &sender,
                        MessageToConnection::TestCountUpResponse { counter },
                    )
                    .await;
            }
        }
    }

    fn topic(&self) -> PubSubTopic {
        format!("{}", self.room_id)
    }

    async fn broadcast(&self, sender: ConnectionID, payload: Bytes, dispatcher: &Dispatcher) {
        for connection_id in self.connections.keys() {
            if *connection_id != sender {
                dispatcher
                    .publish_to_connection(
                        connection_id,
                        MessageToConnection::Broadcast {
                            payload: payload.clone(),
                        },
                    )
                    .await;
            }
        }
    }

    async fn handle_pubsub_message(&self, msg: &PubSubMessage, dispatcher: &Dispatcher) {
        match msg {
            PubSubMessage::Broadcast {
                sender_server,
                sender,
                payload,
            } => {
                let sender_server = ServerID::from_bytes(*sender_server);
                // Ignores messages published by its own server.
                if sender_server == self.server_id {
                    return;
                }
                let sender = ConnectionID::from_bytes(*sender);
                self.broadcast(sender, Bytes::from(payload.to_vec()), dispatcher)
                    .await;
            }
        }
    }
}
