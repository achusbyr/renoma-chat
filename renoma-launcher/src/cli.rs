use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
pub struct Cli {
    #[arg(long, default_value_t = 8080)]
    pub port: u16,
    #[arg(long, default_value = "dist")]
    pub dist_dir: PathBuf,
    #[arg(long, default_value = "renoma.db")]
    pub local_db_path: PathBuf,
    #[arg(long)]
    pub postgres_url: Option<String>,
}
