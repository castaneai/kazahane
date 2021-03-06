#[cfg(test)]
mod tests {
    use kazahane::connections::Connection;
    use kazahane::dispatcher::{Dispatcher, MessageToServer, ServerShutdownReason};
    use kazahane::packets::{
        HelloResponseStatusCode, Packet, RoomNotification, ServerNotification,
    };
    use kazahane::transports::websocket;
    use kazahane::RoomID;
    use std::net::SocketAddr;
    use std::sync::{Arc, Once};
    use tokio::net::TcpListener;
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    struct TestServer {
        server_addr: SocketAddr,
        dispatcher: Arc<Dispatcher>,
    }

    impl TestServer {
        async fn connect(&self) -> impl Connection {
            let url = format!("ws://{}", self.server_addr);
            websocket::connect(url).await.expect("failed to connect")
        }

        async fn connect_and_join(&self, room_id: RoomID) -> impl Connection {
            let mut client = self.connect().await;
            client
                .send(Packet::HelloRequest {
                    token: b"".to_vec(),
                })
                .await
                .unwrap();
            assert!(matches!(
                client.recv().await.unwrap(),
                Packet::HelloResponse {
                    status_code: HelloResponseStatusCode::OK,
                    ..
                }
            ));
            client
                .send(Packet::JoinRoomRequest {
                    room_id: room_id.into_bytes(),
                })
                .await
                .unwrap();
            assert!(matches!(
                client.recv().await.unwrap(),
                Packet::JoinRoomResponse { .. }
            ));
            client
        }

        async fn shutdown(&self) {
            self.dispatcher
                .publish_to_server(MessageToServer::Shutdown {
                    reason: ServerShutdownReason::SigTerm,
                })
                .await;
        }
    }

    async fn spawn_test_server() -> TestServer {
        let redis = redis::Client::open("redis://127.0.0.1").unwrap();
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("failed to bind");
        let addr = listener.local_addr().expect("failed to get local addr");
        let dispatcher = Arc::new(Dispatcher::new());
        let disp = dispatcher.clone();
        tokio::spawn(async move {
            kazahane::server::start(&listener, redis, disp).await;
        });
        TestServer {
            server_addr: addr,
            dispatcher,
        }
    }

    #[tokio::test]
    async fn join_room() {
        init_tracing();

        let server = spawn_test_server().await;
        let room_id = new_random_room_id();
        let _ = server.connect_and_join(room_id).await;
    }

    #[tokio::test]
    async fn broadcast() {
        init_tracing();

        let server = spawn_test_server().await;
        let room_id = new_random_room_id();
        let mut c1 = server.connect_and_join(room_id).await;
        let mut c2 = server.connect_and_join(room_id).await;
        let mut c3 = server.connect_and_join(room_id).await;

        let packet = Packet::BroadcastRequest {
            payload: b"hello".to_vec(),
        };
        c1.send(packet).await.unwrap();

        let resp = c2.recv().await.unwrap();
        assert_eq!(
            resp,
            Packet::RoomNotification(RoomNotification::Broadcast {
                payload: b"hello".to_vec()
            })
        );
        let resp = c3.recv().await.unwrap();
        assert_eq!(
            resp,
            Packet::RoomNotification(RoomNotification::Broadcast {
                payload: b"hello".to_vec()
            })
        );
    }

    #[tokio::test]
    async fn broadcast_beyond_servers() {
        init_tracing();

        let server1 = spawn_test_server().await;
        let server2 = spawn_test_server().await;
        let server3 = spawn_test_server().await;

        let room_id = new_random_room_id();
        let mut c1 = server1.connect_and_join(room_id).await;
        let mut c2 = server2.connect_and_join(room_id).await;
        let mut c3 = server3.connect_and_join(room_id).await;

        let packet = Packet::BroadcastRequest {
            payload: b"hello".to_vec(),
        };
        c1.send(packet).await.unwrap();

        let resp = c2.recv().await.unwrap();
        assert_eq!(
            resp,
            Packet::RoomNotification(RoomNotification::Broadcast {
                payload: b"hello".to_vec()
            })
        );
        let resp = c3.recv().await.unwrap();
        assert_eq!(
            resp,
            Packet::RoomNotification(RoomNotification::Broadcast {
                payload: b"hello".to_vec()
            })
        );
    }

    #[tokio::test]
    async fn room_state() {
        init_tracing();

        let server = spawn_test_server().await;
        let room_id = new_random_room_id();
        let mut c1 = server.connect_and_join(room_id).await;

        c1.send(Packet::TestCountUp {}).await.unwrap();
        let resp = c1.recv().await.unwrap();
        assert_eq!(resp, Packet::TestCountUpResponse { counter: 1 });

        let mut c2 = server.connect_and_join(room_id).await;
        c2.send(Packet::TestCountUp {}).await.unwrap();
        let resp = c2.recv().await.unwrap();
        assert_eq!(resp, Packet::TestCountUpResponse { counter: 2 });

        let another_room_id = new_random_room_id();
        let mut c3 = server.connect_and_join(another_room_id).await;
        c3.send(Packet::TestCountUp {}).await.unwrap();
        let resp = c3.recv().await.unwrap();
        assert_eq!(resp, Packet::TestCountUpResponse { counter: 1 });
    }

    #[tokio::test]
    async fn room_live_migration() {
        init_tracing();

        let server1 = spawn_test_server().await;
        let server2 = spawn_test_server().await;

        let room_id = new_random_room_id();

        let mut c1 = server1.connect_and_join(room_id).await;
        c1.send(Packet::TestCountUp {}).await.unwrap();
        let resp = c1.recv().await.unwrap();
        assert_eq!(resp, Packet::TestCountUpResponse { counter: 1 });

        let mut c2 = server1.connect_and_join(room_id).await;
        c2.send(Packet::TestCountUp {}).await.unwrap();
        let resp = c2.recv().await.unwrap();
        assert_eq!(resp, Packet::TestCountUpResponse { counter: 2 });

        server1.shutdown().await;

        assert_eq!(
            c1.recv().await.unwrap(),
            Packet::ServerNotification(ServerNotification::Shutdown)
        );
        assert_eq!(
            c2.recv().await.unwrap(),
            Packet::ServerNotification(ServerNotification::Shutdown)
        );

        let mut c1_next = server2.connect_and_join(room_id).await;
        c1_next.send(Packet::TestCountUp {}).await.unwrap();
        let resp = c1_next.recv().await.unwrap();
        assert_eq!(resp, Packet::TestCountUpResponse { counter: 3 });
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
