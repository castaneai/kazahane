use std::net::{SocketAddr};
use anyhow::{anyhow, bail, Context};
use async_trait::async_trait;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use crate::packets::KazahanePacket;
use crate::server::{Connection};
use binrw::{BinRead, BinWrite};
use binrw::io::Cursor;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

type WsStream = WebSocketStream<TcpStream>;

pub(crate) async fn accept(raw_stream: TcpStream, _: SocketAddr) -> crate::Result<impl Connection> {
    let ws_stream = tokio_tungstenite::accept_async(raw_stream)
        .await
        .context("failed to accept as websocket")?;
    Ok(WebSocketConnection::new(ws_stream))
}

#[derive(Debug)]
pub struct WebSocketConnection {
    sender: SplitSink<WsStream, Message>,
    receiver: SplitStream<WsStream>,
}

impl WebSocketConnection {
    pub fn new(ws: WsStream) -> Self {
        let (sender, receiver) = ws.split();
        Self {
            sender,
            receiver,
        }
    }
}

#[async_trait]
impl Connection for WebSocketConnection {
    async fn send(&mut self, packet: &KazahanePacket) -> crate::Result<()> {
        let mut writer = Cursor::new(Vec::new());
        packet.write_to(&mut writer).context("failed to write packet")?;
        self.sender.send(Message::Binary(writer.into_inner()))
            .await
            .context("failed to send message")
    }

    async fn recv(&mut self) -> crate::Result<KazahanePacket> {
        loop {
            let msg = self.receiver.next()
                .await
                .ok_or(anyhow!("stream closed"))?
                .context("failed to receive")?;
            match msg {
                Message::Binary(data) => {
                    let mut cursor = Cursor::new(data);
                    return KazahanePacket::read(&mut cursor).context("failed to parse");
                },
                Message::Ping(_) => continue,
                Message::Pong(_) => continue,
                Message::Close(_) => bail!("websocket closed"),
                _ => bail!("invalid websocket message"),
            }
        }
    }
}
