use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(about = "Renoma utilies - trunk must be installed")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    #[command(about = "Launch the app")]
    Launch,
    #[command(about = "Build the app")]
    Dist { target_triple: Option<String> },
}
