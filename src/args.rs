use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Clone, Debug, Parser)]
pub struct Args {
    #[arg(short, long)]
    pub verbose: bool,

    #[arg(short, long, default_value = ".")]
    pub repo: PathBuf,
    /// The silo command to execute
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Clone, Debug, Subcommand)]
pub enum Command {
    /// Initialize a silo repository
    Init,
    /// Applies the configuration stored in a silo repo
    Apply,
}
