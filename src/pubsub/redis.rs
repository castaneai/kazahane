use crate::pubsub::{PubSub, Subscription, Topic};
use anyhow::Context;
use async_trait::async_trait;
use bytes::Bytes;
use futures::StreamExt;
use redis::AsyncCommands;

struct RedisPubSub {
    client: redis::Client,
}

impl RedisPubSub {
    pub fn new(client: redis::Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl PubSub for RedisPubSub {
    async fn publish(&self, topic: Topic, data: Bytes) -> crate::Result<()> {
        let mut conn = self
            .client
            .get_async_connection()
            .await
            .context("failed to get conn")?;
        conn.publish(topic.as_str(), data.to_vec())
            .await
            .context("failed to publish")
    }

    async fn subscribe(&self, topic: Topic) -> crate::Result<Box<dyn Subscription>> {
        let conn = self
            .client
            .get_async_connection()
            .await
            .context("failed to get conn")?;
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
        let pubsub = RedisPubSub::new(client);
        let mut sub = pubsub.subscribe("test".to_string()).await.unwrap();
        pubsub
            .publish("test".to_string(), Bytes::from("hello"))
            .await
            .unwrap();
        let msg = sub.next_message().await.unwrap();
        assert_eq!(msg, Some(Bytes::from("hello")));
    }
}
