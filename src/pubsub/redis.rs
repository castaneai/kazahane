use crate::pubsub::{PubSub, Subscription, Topic};
use anyhow::Context;
use async_trait::async_trait;
use bytes::Bytes;
use futures::StreamExt;
use redis::AsyncCommands;

pub(crate) struct RedisPubSub {
    client: redis::Client,
    pub_conn: redis::aio::Connection,
}

impl RedisPubSub {
    pub async fn new(client: redis::Client) -> crate::Result<Self> {
        let pub_conn = client
            .get_async_connection()
            .await
            .context("failed to connect to Redis")?;
        Ok(Self { client, pub_conn })
    }
}

#[async_trait]
impl PubSub for RedisPubSub {
    async fn publish(&mut self, topic: Topic, data: Bytes) -> crate::Result<()> {
        self.pub_conn
            .publish(topic.as_str(), data.to_vec())
            .await
            .context("failed to publish")
    }

    async fn subscribe(&mut self, topic: Topic) -> crate::Result<Box<dyn Subscription>> {
        let conn = self
            .client
            .get_async_connection()
            .await
            .context("failed to create subscribe connection")?;
        let mut pubsub = conn.into_pubsub();
        pubsub
            .subscribe(topic)
            .await
            .context("failed to subscribe")?;
        Ok(Box::new(RedisSubscription { pubsub }))
    }
}

struct RedisSubscription {
    pubsub: redis::aio::PubSub,
}

#[async_trait]
impl Subscription for RedisSubscription {
    async fn next_message(&mut self) -> crate::Result<Option<Bytes>> {
        let msg = self.pubsub.on_message().next().await;
        match msg {
            Some(msg) => {
                let payload: Vec<u8> = msg.get_payload().context("failed to get payload")?;
                Ok(Some(Bytes::from(payload)))
            }
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::pubsub::redis::RedisPubSub;
    use crate::pubsub::PubSub;
    use bytes::Bytes;

    #[tokio::test]
    async fn test_redis() {
        let client = redis::Client::open("redis://127.0.0.1").unwrap();
        let mut pubsub = RedisPubSub::new(client).await.unwrap();
        let mut sub = pubsub.subscribe("test".to_string()).await.unwrap();
        pubsub
            .publish("test".to_string(), Bytes::from("hello"))
            .await
            .unwrap();
        let msg = sub.next_message().await.unwrap();
        assert_eq!(msg, Some(Bytes::from("hello")));
    }
}
