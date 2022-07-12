use crate::connections::connection_task;
use crate::rooms::RoomMap;
use crate::transports::websocket;
use crate::types::ConnectionToServer;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tracing::debug;

pub async fn start(listener: &TcpListener) {
    let _rooms = RoomMap::new();
    // TODO: buffer size
    let (sender_to_server, mut rx) = mpsc::channel(8);
    loop {
        tokio::select! {
            Ok(conn) = websocket::accept(listener) => {
                tokio::spawn(connection_task(conn, sender_to_server.clone()));
            }
            Some(msg) = rx.recv() => {
                handle_connection_to_server(msg).await;
            }
            else => break
        }
    }
}

async fn handle_connection_to_server(msg: ConnectionToServer) {
    debug!("handle connection to server: {:?}", msg);
}
