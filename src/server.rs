use crate::packets::KazahanePacket;
use crate::transports::websocket;
use async_trait::async_trait;
use tokio::net::TcpListener;
use tracing::{debug, error};

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
                debug!("packet received: {:?}", packet);
            }
            Err(e) => {
                error!("failed to receive packet: {:?}", e);
                break;
            }
        }
    }
}
