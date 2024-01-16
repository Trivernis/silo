use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Clone, Debug, Parser)]
pub struct Args {
    #[arg(short, long)]
    pub verbose: bool,

    #[arg(short, long, default_value = default_repo() )]
    pub repo: PathBuf,
    /// The silo command to execute
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Clone, Debug, Subcommand)]
pub enum Command {
    /// Initialize a silo repository
    Init(InitArgs),
    /// Applies the configuration stored in a silo repo
    Apply,

    /// Print the entire context available to templates
    Context,

    /// Print the path of the repo
    Repo,
}

#[derive(Clone, Debug, Parser)]
pub struct InitArgs {
    /// Init using a remote repository
    #[arg()]
    pub remote: Option<String>,
}

fn default_repo() -> &'static str {
    lazy_static::lazy_static! {
        static ref DEFAULT_REPO: String = dirs::data_dir()
        .unwrap()
        .join("silo")
        .to_string_lossy()
        .into();
    }
    &*DEFAULT_REPO
}
