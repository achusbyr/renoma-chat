mod cli;

use axum::Router;
use clap::Parser;
use std::net::SocketAddr;
use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();
    let cli = cli::Cli::parse();
    let router =
        Router::new().fallback_service(ServeDir::new(cli.dist_dir.unwrap_or("dist".into())));
    let addr = SocketAddr::from(([127, 0, 0, 1], cli.port.unwrap_or(8080)));
    tracing::info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    let router = backend::init(router);
    axum::serve(listener, router).await?;
    Ok(())
}
