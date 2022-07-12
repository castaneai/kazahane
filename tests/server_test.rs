use kazahane::connections::Connection;
use kazahane::packets::{HelloRequestPacket, JoinRoomRequestPacket, KazahanePacket, PacketType};
use kazahane::transports::websocket;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

struct TestServer {
    server_addr: SocketAddr,
}

impl TestServer {
    async fn connect(&self) -> impl Connection {
        let url = format!("ws://{}", self.server_addr);
        websocket::connect(url).await.expect("failed to connect")
    }
}

async fn test_server() -> TestServer {
    init_tracing();
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind");
    let addr = listener.local_addr().expect("failed to get local addr");
    tokio::spawn(async move {
        kazahane::server::start(&listener).await;
    });
    TestServer { server_addr: addr }
}

#[tokio::test]
async fn hello_world() {
    let s = test_server().await;
    let mut client = s.connect().await;

    let p = KazahanePacket::new(PacketType::HelloRequest, HelloRequestPacket {}).unwrap();

    client.send(&p).await.expect("failed to send");
    let resp = client.recv().await.expect("failed to recv");
    assert_eq!(PacketType::HelloResponse, resp.packet_type);
}

fn init_tracing() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "kazahane=debug,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();
}
