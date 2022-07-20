pub mod redis;

use anyhow::Context;
use async_trait::async_trait;
use binrw::{binrw, BinRead, BinWrite};
use bytes::Bytes;
use std::fmt::Debug;
use std::io::Cursor;

pub(crate) type PubSubTopic = String;

#[async_trait]
pub(crate) trait PubSub {
    async fn publish(&mut self, topic: PubSubTopic, msg: PubSubMessage) -> crate::Result<()>;
    async fn subscribe(
        &mut self,
        topic: PubSubTopic,
    ) -> crate::Result<Box<dyn Subscription + Send>>;
}

#[async_trait]
pub(crate) trait Subscription {
    async fn next_message(&mut self) -> crate::Result<Option<Bytes>>;
}

#[binrw]
#[brw(little)]
#[derive(Debug, PartialEq)]
pub enum PubSubMessage {
    #[brw(magic = 0x01u8)]
    Broadcast {
        sender_server: uuid::Bytes,
        sender: uuid::Bytes,
        #[br(temp)]
        #[bw(calc = payload.len() as u16)]
        payload_size: u16,
        #[br(count = payload_size)]
        payload: Vec<u8>,
    },
}

impl PubSubMessage {
    pub fn from_bytes(bytes: Bytes) -> crate::Result<Self> {
        let mut cursor = Cursor::new(bytes);
        Self::read(&mut cursor).context("failed to parse pubsub message")
    }

    pub fn to_bytes(&self) -> Bytes {
        let mut writer = Cursor::new(Vec::new());
        self.write_to(&mut writer).unwrap();
        Bytes::from(writer.into_inner())
    }
}
