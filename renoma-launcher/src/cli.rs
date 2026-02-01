use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
pub struct Cli {
    #[arg(short, long)]
    pub port: Option<u16>,
    #[arg(short, long)]
    pub dist_dir: Option<PathBuf>,
}
