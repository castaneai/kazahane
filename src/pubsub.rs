pub mod redis;

use crate::types::ConnectionID;
use anyhow::{bail, Context};
use async_trait::async_trait;
use binrw::{binrw, BinRead, BinWrite};
use bytes::Bytes;
use std::fmt::Debug;
use std::io::Cursor;

pub(crate) type PubSubTopic = String;

#[async_trait]
pub(crate) trait PubSub {
    async fn publish<T>(&mut self, topic: PubSubTopic, msg: T) -> crate::Result<()>
    where
        T: Debug + Send + IntoPubSubMessage;
    async fn subscribe(
        &mut self,
        topic: PubSubTopic,
    ) -> crate::Result<Box<dyn Subscription + Send>>;
}

#[async_trait]
pub(crate) trait Subscription {
    async fn next_message(&mut self) -> crate::Result<Option<Bytes>>;
}

pub trait IntoPubSubMessage {
    fn into_pubsub_message(self) -> crate::Result<PubSubMessage>;
}

#[derive(Debug)]
#[binrw]
#[brw(little)]
pub struct PubSubMessage {
    pub message_type: MessageType,
    pub payload_size: u16,
    #[br(count = payload_size)]
    pub payload: Vec<u8>,
}

impl PubSubMessage {
    pub fn from_bytes(data: Bytes) -> Self {
        let mut buf = Cursor::new(data);
        Self::read(&mut buf).unwrap()
    }
}

#[derive(Debug, PartialEq, Eq)]
#[binrw]
#[brw(repr = u8)]
#[repr(u8)]
pub enum MessageType {
    Broadcast = 0x01,
}

#[derive(Debug)]
#[binrw]
#[brw(little)]
pub struct PubSubPayloadBroadcast {
    pub sender: uuid::Bytes,
    pub payload_size: u16,
    #[br(count = payload_size)]
    pub payload: Vec<u8>,
}

impl PubSubPayloadBroadcast {
    pub fn new(sender: ConnectionID, payload: impl Into<Vec<u8>>) -> Self {
        let vec = payload.into();
        Self {
            sender: sender.into_bytes(),
            payload_size: vec.len() as u16,
            payload: vec,
        }
    }
}

impl IntoPubSubMessage for PubSubPayloadBroadcast {
    fn into_pubsub_message(self) -> crate::Result<PubSubMessage> {
        PubSubMessage::new(MessageType::Broadcast, self)
    }
}

impl PubSubMessage {
    pub fn new<T: BinWrite>(message_type: MessageType, bw: T) -> crate::Result<Self>
    where
        T::Args: Default,
    {
        let mut writer = Cursor::new(Vec::new());
        bw.write_to(&mut writer)
            .context("failed to write to pubsub message")?;
        let vec = writer.into_inner();
        Ok(Self {
            message_type,
            payload_size: vec.len() as u16,
            payload: vec,
        })
    }

    pub fn to_vec(&self) -> crate::Result<Vec<u8>> {
        let mut writer = Cursor::new(Vec::new());
        self.write_to(&mut writer)
            .context("failed to write pubsub message")?;
        Ok(writer.into_inner())
    }

    pub fn parse_as_broadcast(&self) -> crate::Result<PubSubPayloadBroadcast> {
        if self.message_type != MessageType::Broadcast {
            bail!("must be broadcast type")
        }
        self.parse_payload()
    }

    pub fn parse_payload<T>(&self) -> crate::Result<T>
    where
        T: binrw::BinRead,
        T::Args: Default,
    {
        let mut cursor = Cursor::new(&self.payload);
        T::read(&mut cursor).context("failed to parse")
    }
}
