use crate::dispatcher::{Dispatcher, MessageToConnection, MessageToRoom};
use crate::room_states::{RoomStateStore, StateData};
use crate::types::{ConnectionID, RoomID};
use core::convert::TryInto;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::debug;

pub(crate) async fn room_task(
    room_id: RoomID,
    mut receiver: mpsc::Receiver<MessageToRoom>,
    dispatcher: Arc<Dispatcher>,
    mut state: impl RoomStateStore,
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
                room.handle_message(msg, &dispatcher, &mut state).await;
            }
            // TODO: else => break 入れるとなぜかすぐにroom dropされてしまう
            // senderがまだ残ってるはずなのに……。
            // else => break
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
            MessageToRoom::Broadcast { payload, .. } => {
                for connection_id in self.connections.keys() {
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
}
