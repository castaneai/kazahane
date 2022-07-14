#[cfg(test)]
mod tests {
    use kazahane::connections::Connection;
    use kazahane::packets::{BroadcastMessagePacket, JoinRoomRequestPacket, PacketType};
    use kazahane::transports::websocket;
    use kazahane::RoomID;
    use std::net::SocketAddr;
    use std::sync::Once;
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

    async fn spawn_test_server() -> TestServer {
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
    async fn join_room() {
        init_tracing();

        let server = spawn_test_server().await;
        let mut c1 = server.connect().await;

        let room_id = new_random_room_id();
        let req = JoinRoomRequestPacket::new(room_id);
        c1.send(req).await.unwrap();
        let resp = c1.recv().await.unwrap();
        assert_eq!(PacketType::JoinRoomResponse, resp.packet_type);

        let packet = BroadcastMessagePacket::new(c1.connection_id(), room_id, "hello");
        c1.send(packet).await.unwrap();
        let resp = c1.recv().await.unwrap();
        assert_eq!(PacketType::BroadcastMessage, resp.packet_type);
    }

    static LOGGER_INIT: Once = Once::new();

    fn init_tracing() {
        LOGGER_INIT.call_once(|| {
            tracing_subscriber::registry()
                .with(tracing_subscriber::EnvFilter::new(
                    std::env::var("RUST_LOG")
                        .unwrap_or_else(|_| "kazahane=debug,tower_http=debug".into()),
                ))
                .with(tracing_subscriber::fmt::layer())
                .init();
        });
    }

    pub(crate) fn new_random_room_id() -> RoomID {
        RoomID::new_v4()
    }
}
