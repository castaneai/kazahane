use envconfig::Envconfig;
use kazahane::server;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Envconfig)]
pub struct Config {
    #[envconfig(from = "PORT", default = "8080")]
    pub listen_port: u16,
}

#[tokio::main]
async fn main() {
    init_tracing();
    let config: Config = Config::init_from_env().unwrap();
    info!(?config);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.listen_port));
    let listener = TcpListener::bind(&addr).await.expect("failed to bind");
    server::start(&listener).await;
}

fn init_tracing() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "kazahane=debug,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();
}
