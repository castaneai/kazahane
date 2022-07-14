mod redis;

use async_trait::async_trait;
use bytes::Bytes;

type Topic = String;

#[async_trait]
pub(crate) trait PubSub {
    async fn publish(&mut self, topic: Topic, data: Bytes) -> crate::Result<()>;
    async fn subscribe(&mut self, topic: Topic) -> crate::Result<Box<dyn Subscription>>;
}

#[async_trait]
pub(crate) trait Subscription {
    async fn next_message(&mut self) -> crate::Result<Option<Bytes>>;
}
