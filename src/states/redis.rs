use crate::states::{StateData, StateKey, StateStore};
use anyhow::Context;
use async_trait::async_trait;
use redis::AsyncCommands;

pub(crate) struct RedisStateStore {
    conn: redis::aio::Connection,
}

impl RedisStateStore {
    pub fn new(conn: redis::aio::Connection) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl StateStore for RedisStateStore {
    async fn get_state(&mut self, key: StateKey) -> crate::Result<Option<StateData>> {
        self.conn.get(key).await.context("failed to get")
    }

    async fn save_state(&mut self, key: StateKey, data: StateData) -> crate::Result<()> {
        self.conn.set(key, data).await.context("failed to set")
    }

    async fn delete_state(&mut self, key: StateKey) -> crate::Result<()> {
        self.conn.del(key).await.context("failed to delete")
    }
}

#[cfg(test)]
mod tests {
    use crate::states::redis::RedisStateStore;
    use crate::states::{StateData, StateStore};

    #[tokio::test]
    async fn test_redis() {
        let client = redis::Client::open("redis://127.0.0.1").unwrap();
        let conn = client.get_async_connection().await.unwrap();
        let mut store = RedisStateStore::new(conn);
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
