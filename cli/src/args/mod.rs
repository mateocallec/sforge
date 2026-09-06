pub mod request_args;
pub mod session_args;

use clap::{Parser, Subcommand};

use request_args::RequestArgs;
use session_args::SessionCommand;

#[derive(Debug, Parser)]
#[command(
    name = "sforge",
    version,
    about = "A curl-like HTTP client with named, reusable sessions."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<TopCommand>,

    #[command(flatten)]
    pub request: RequestArgs,
}

#[derive(Debug, Subcommand)]
pub enum TopCommand {
    /// Manage named sessions (create/update, list, delete), or browse them
    /// interactively when run with no further subcommand.
    Session {
        #[command(subcommand)]
        command: Option<SessionCommand>,
    },
}
