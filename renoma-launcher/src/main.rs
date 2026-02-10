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
    let router = Router::new().fallback_service(ServeDir::new(cli.dist_dir));
    let addr = SocketAddr::from(([0, 0, 0, 0], cli.port));
    tracing::info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    let config = if let Some(url) = cli.postgres_url {
        backend::DatabaseConfig::Postgres { url }
    } else {
        let db_url = format!("sqlite:{}?mode=rwc", cli.local_db_path.display());
        backend::DatabaseConfig::Local { url: db_url }
    };
    let router = backend::init(router, config).await;
    axum::serve(listener, router).await?;
    Ok(())
}
