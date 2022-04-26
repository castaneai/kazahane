use std::net::{SocketAddr};
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::sleep;
use tokio_tungstenite::{client_async};
use kazahane::packets::{KazahanePacket, PacketType};
use kazahane::server::Connection;
use kazahane::transports::websocket::WebSocketConnection;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

struct TestServer {
    server_addr: SocketAddr,
}

impl TestServer {
    async fn connect(&self) -> impl Connection {
        let conn = TcpStream::connect(&self.server_addr).await.expect("failed to connect to test server");
        let url = format!("ws://{}", self.server_addr.to_string());
        let (stream, _) = client_async(url, conn).await.expect("failed to connect");
        WebSocketConnection::new(stream)
    }
}

async fn test_server() -> TestServer {
    init_tracing();
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("failed to bind");
    let addr = listener.local_addr().expect("failed to get local addr");
    tokio::spawn(async move {
        kazahane::server::start(&listener).await;
    });
    TestServer {
        server_addr: addr,
    }
}

#[tokio::test]
async fn hello_world() {
    let s = test_server().await;
    let mut client = s.connect().await;

    let p = KazahanePacket {
        packet_type: PacketType::HelloRequest,
        payload_size: 0,
        payload: b"hello".to_vec(),
    };
    client.send(&p).await.expect("failed to send");
    sleep(Duration::from_secs(3)).await;
}

fn init_tracing() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "kazahane=debug,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();
}
