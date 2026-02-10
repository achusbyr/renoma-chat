use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(about = "Renoma utilities - trunk must be installed and available in PATH")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    #[command(about = "Launch the app")]
    Launch { extra_arguments: Vec<String> },
    #[command(about = "Build the app")]
    Dist { target_triple: Option<String> },
}
