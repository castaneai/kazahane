use crate::pubsub::{PubSub, PubSubMessage, PubSubTopic, Subscription};
use anyhow::Context;
use async_trait::async_trait;
use bytes::Bytes;
use futures::StreamExt;
use redis::AsyncCommands;
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
    async fn publish(&mut self, topic: PubSubTopic, msg: PubSubMessage) -> crate::Result<()> {
        debug!("publish to pubsub (topic: {}, data: {:?})", topic, msg);
        self.pub_conn
            .publish(topic.as_str(), msg.to_bytes().to_vec())
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
    use crate::pubsub::{PubSub, PubSubMessage};
    use crate::types::{ConnectionID, ServerID};

    #[tokio::test]
    async fn test_redis() {
        let client = redis::Client::open("redis://127.0.0.1").unwrap();
        let conn = client.get_tokio_connection_manager().await.unwrap();
        let mut pubsub = RedisPubSub::new(client, conn);
        let mut sub = pubsub.subscribe("test".to_string()).await.unwrap();
        let sender_server = ServerID::new_v4();
        let sender = ConnectionID::new_v4();
        let msg = PubSubMessage::Broadcast {
            sender_server: sender_server.into_bytes(),
            sender: sender.into_bytes(),
            payload: b"hello".to_vec(),
        };
        pubsub.publish("test".to_string(), msg).await.unwrap();
        let msg = PubSubMessage::from_bytes(sub.next_message().await.unwrap().unwrap()).unwrap();
        assert_eq!(
            msg,
            PubSubMessage::Broadcast {
                sender_server: sender_server.into_bytes(),
                sender: sender.into_bytes(),
                payload: b"hello".to_vec()
            }
        );
    }
}
