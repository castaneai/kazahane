use crate::pubsub::{IntoPubSubMessage, PubSub, PubSubTopic, Subscription};
use anyhow::Context;
use async_trait::async_trait;
use bytes::Bytes;
use futures::StreamExt;
use redis::AsyncCommands;
use std::fmt::Debug;
use tracing::debug;

pub(crate) struct RedisPubSub {
    client: redis::Client,
    pub_conn: redis::aio::ConnectionManager,
}

impl RedisPubSub {
    pub fn new(client: redis::Client, conn: redis::aio::ConnectionManager) -> Self {
        Self {
            client,
            pub_conn: conn,
        }
    }
}

#[async_trait]
impl PubSub for RedisPubSub {
    async fn publish<T>(&mut self, topic: PubSubTopic, msg: T) -> crate::Result<()>
    where
        T: Debug + Send + IntoPubSubMessage,
    {
        debug!("publish to pubsub (topic: {}, data: {:?})", topic, msg);
        self.pub_conn
            .publish(
                topic.as_str(),
                msg.into_pubsub_message().unwrap().to_vec().unwrap(),
            )
            .await
            .context("failed to publish")
    }

    async fn subscribe(
        &mut self,
        topic: PubSubTopic,
    ) -> crate::Result<Box<dyn Subscription + Send>> {
        debug!("start subscribe from pubsub (topic: {})", topic);
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
    use crate::pubsub::{PubSub, PubSubMessage, PubSubPayloadBroadcast};
    use crate::types::ConnectionID;

    #[tokio::test]
    async fn test_redis() {
        let client = redis::Client::open("redis://127.0.0.1").unwrap();
        let conn = client.get_tokio_connection_manager().await.unwrap();
        let mut pubsub = RedisPubSub::new(client, conn);
        let mut sub = pubsub.subscribe("test".to_string()).await.unwrap();
        let sender = ConnectionID::new_v4();
        let msg = PubSubPayloadBroadcast::new(sender, b"hello".to_vec());
        pubsub.publish("test".to_string(), msg).await.unwrap();
        let msg = PubSubMessage::from_bytes(sub.next_message().await.unwrap().unwrap());
        assert_eq!(msg.parse_as_broadcast().unwrap().payload, b"hello");
    }
}
