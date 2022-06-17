use anyhow::Context;
use crate::packets::{HelloResponsePacket, HelloResponseStatusCode, KazahanePacket, PacketType};
use crate::transports::websocket;
use async_trait::async_trait;
use tokio::net::TcpListener;
use tracing::{error, warn};

pub async fn start(listener: &TcpListener) {
    while let Ok((stream, addr)) = listener.accept().await {
        match websocket::accept(stream, addr).await {
            Ok(conn) => {
                tokio::spawn(async move {
                    handle(conn).await;
                });
            }
            Err(err) => error!("failed to accept: {:?}", err),
        }
    }
}

#[async_trait]
pub trait Connection {
    async fn send(&mut self, packet: &KazahanePacket) -> crate::Result<()>;
    async fn recv(&mut self) -> crate::Result<KazahanePacket>;
}

async fn handle(mut conn: impl Connection) {
    loop {
        match conn.recv().await {
            Ok(packet) => {
                match packet.packet_type {
                    PacketType::HelloRequest => {
                        if let Err(err) = handle_hello_request(&mut conn).await {
                            error!("failed to handle hello request: {:?}", err)
                        }
                    }
                    _ => {
                        warn!("unknown packet received: {:?}", packet);
                    }
                }
            }
            Err(e) => {
                error!("failed to receive packet: {:?}", e);
                break;
            }
        }
    }
}

async fn handle_hello_request(conn: &mut impl Connection) -> crate::Result<()> {
    let resp = HelloResponsePacket::new(HelloResponseStatusCode::Denied, "unimplemented");
    let packet = KazahanePacket::new(&resp).context("failed to create hello response packet")?;
    conn.send(&packet).await.context("failed to send hello response")
}
