use axum::extract::ws::{Message, WebSocket};
use axum::extract::WebSocketUpgrade;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use envconfig::Envconfig;
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub mod packets;

#[derive(Envconfig)]
pub struct Config {
    #[envconfig(from = "PORT", default = "8080")]
    pub listen_port: u16,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "kazahane=debug,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();
    let config: Config = Config::init_from_env().unwrap();

    let addr = SocketAddr::from(([0, 0, 0, 0], config.listen_port));
    let app = Router::new().route("/", get(ws_handler));

    tracing::info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    if let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Binary(data) => {
                    eprintln!("client sent: {:?}", data);
                }
                Message::Close(_) => {
                    eprintln!("client sent close");
                }
                _ => {}
            }
        } else {
            eprintln!("client disconnected");
            return;
        }
    }
}
