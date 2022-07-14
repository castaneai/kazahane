mod redis;

use async_trait::async_trait;

pub type StateKey = String;
pub type StateData = Vec<u8>;

#[async_trait]
pub trait StateStore {
    async fn get_state(&mut self, key: StateKey) -> crate::Result<Option<StateData>>;
    async fn save_state(&mut self, key: StateKey, data: StateData) -> crate::Result<()>;
    async fn delete_state(&mut self, key: StateKey) -> crate::Result<()>;
}
