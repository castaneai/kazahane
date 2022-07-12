use crate::connections::Connection;
use crate::packets::KazahanePacket;
use crate::types::ConnectionID;
use anyhow::{anyhow, bail, Context};
use async_trait::async_trait;
use binrw::io::Cursor;
use binrw::{BinRead, BinWrite};
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;
use uuid::Uuid;

pub async fn connect(url: impl IntoClientRequest + Unpin) -> crate::Result<impl Connection> {
    let (ws_stream, _) = tokio_tungstenite::connect_async(url)
        .await
        .context("failed to connect via websocket")?;
    Ok(WebSocketConnection::new(ws_stream))
}

pub(crate) async fn accept(listener: &TcpListener) -> crate::Result<impl Connection> {
    let (stream, _) = listener
        .accept()
        .await
        .context("failed to accept TCP connection")?;
    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .context("failed to accept as websocket")?;
    Ok(WebSocketConnection::new(ws_stream))
}

#[derive(Debug)]
pub(crate) struct WebSocketConnection<S> {
    connection_id: ConnectionID,
    sender: SplitSink<WebSocketStream<S>, Message>,
    receiver: SplitStream<WebSocketStream<S>>,
}

impl<S> WebSocketConnection<S> {
    fn new(ws: WebSocketStream<S>) -> Self
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        let (sender, receiver) = ws.split();
        Self {
            connection_id: Uuid::new_v4(),
            sender,
            receiver,
        }
    }
}

#[async_trait]
impl<S> Connection for WebSocketConnection<S>
where
    S: Send + AsyncRead + AsyncWrite + Unpin,
{
    fn connection_id(&self) -> ConnectionID {
        self.connection_id
    }

    async fn send(&mut self, packet: &KazahanePacket) -> crate::Result<()> {
        let mut writer = Cursor::new(Vec::new());
        packet
            .write_to(&mut writer)
            .context("failed to write packet")?;
        self.sender
            .send(Message::Binary(writer.into_inner()))
            .await
            .context("failed to send message")
    }

    async fn recv(&mut self) -> crate::Result<KazahanePacket> {
        loop {
            let msg = self
                .receiver
                .next()
                .await
                .ok_or_else(|| anyhow!("stream closed"))?
                .context("failed to receive")?;
            match msg {
                Message::Binary(data) => {
                    let mut cursor = Cursor::new(data);
                    return KazahanePacket::read(&mut cursor).context("failed to parse");
                }
                Message::Ping(_) => continue,
                Message::Pong(_) => continue,
                Message::Close(_) => bail!("websocket closed"),
                _ => bail!("invalid websocket message"),
            }
        }
    }
}
