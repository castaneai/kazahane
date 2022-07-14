use crate::room_states::{RoomStateStore, StateData, StateKey};
use crate::types::RoomID;
use anyhow::Context;
use async_trait::async_trait;
use redis::AsyncCommands;

pub(crate) struct RedisStateStore {
    room_id: RoomID,
    conn: redis::aio::ConnectionManager,
}

impl RedisStateStore {
    pub fn new(room_id: RoomID, conn: redis::aio::ConnectionManager) -> Self {
        Self { room_id, conn }
    }

    fn redis_key(&self, key: StateKey) -> String {
        format!("{}/{}", self.room_id, key)
    }
}

#[async_trait]
impl RoomStateStore for RedisStateStore {
    async fn get_state(&mut self, key: StateKey) -> crate::Result<Option<StateData>> {
        self.conn
            .get(self.redis_key(key))
            .await
            .context("failed to get")
    }

    async fn save_state(&mut self, key: StateKey, data: StateData) -> crate::Result<()> {
        self.conn
            .set(self.redis_key(key), data)
            .await
            .context("failed to set")
    }

    async fn delete_state(&mut self, key: StateKey) -> crate::Result<()> {
        self.conn
            .del(self.redis_key(key))
            .await
            .context("failed to delete")
    }
}

#[cfg(test)]
mod tests {
    use crate::room_states::redis::RedisStateStore;
    use crate::room_states::{RoomStateStore, StateData};
    use crate::types::RoomID;

    #[tokio::test]
    async fn test_redis() {
        let client = redis::Client::open("redis://127.0.0.1").unwrap();
        let conn = client.get_tokio_connection_manager().await.unwrap();
        let room_id = RoomID::new_v4();
        let mut store = RedisStateStore::new(room_id, conn);
        let missing = store.get_state("test".to_string()).await.unwrap();
        assert_eq!(missing, None);
        store
            .save_state("test".to_string(), "hello".into())
            .await
            .unwrap();
        let data = store.get_state("test".to_string()).await.unwrap();
        assert_eq!(data, Some(StateData::from("hello")));
        store.delete_state("test".to_string()).await.unwrap();
        let missing = store.get_state("test".to_string()).await.unwrap();
        assert_eq!(missing, None);
    }
}
