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
    unsafe {
        std::env::set_var(
            "LOCAL_DB_PATH",
            cli.local_db_path.unwrap_or("db.json".into()),
        )
    };
    let router = backend::init(router);
    axum::serve(listener, router).await?;
    Ok(())
}
