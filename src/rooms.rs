use crate::dispatcher::{Dispatcher, MessageToConnection, MessageToRoom};
use crate::pubsub::{MessageType, PubSub, PubSubMessage, PubSubPayloadBroadcast, PubSubTopic};
use crate::room_states::{RoomStateStore, StateData};
use crate::types::{ConnectionID, RoomID};
use bytes::Bytes;
use core::convert::TryInto;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error};

pub(crate) async fn room_task(
    room_id: RoomID,
    mut receiver: mpsc::Receiver<MessageToRoom>,
    dispatcher: Arc<Dispatcher>,
    mut state: impl RoomStateStore,
    mut pubsub: impl PubSub,
) {
    debug!("start room task (room_id: {})", room_id);
    let mut room = Room {
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
                let msg = PubSubMessage::from_bytes(msg);
                room.handle_pubsub_message(&msg, &dispatcher).await;
            }
            else => break
        }
    }
    debug!("drop room: {}", room_id);
    dispatcher.drop_room(&room_id);
}

#[derive(Debug)]
struct Room {
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
                let msg = PubSubPayloadBroadcast::new(sender, payload.to_vec());
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
        match msg.message_type {
            MessageType::Broadcast => {
                // TODO: error handling
                let msg = msg.parse_as_broadcast().unwrap();
                let sender = ConnectionID::from_bytes(msg.sender);
                self.broadcast(sender, Bytes::from(msg.payload), dispatcher)
                    .await;
            }
        }
    }
}
