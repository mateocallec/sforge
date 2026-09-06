use clap::Parser;

use sforge::args::session_args::SessionCommand;
use sforge::args::{Cli, TopCommand};
use sforge::config::Config;
use sforge::error::Result;

fn main() {
    let cli = Cli::parse();

    if let Err(err) = run(cli) {
        eprintln!("sforge: {err}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    let config = Config::load()?;

    match cli.command {
        Some(TopCommand::Session { command }) => match command {
            Some(SessionCommand::Setup(args)) => {
                sforge::commands::session_setup::run(&config, args)
            }
            Some(SessionCommand::List(args)) => sforge::commands::session_list::run(&config, args),
            Some(SessionCommand::Delete(args)) => {
                sforge::commands::session_delete::run(&config, args)
            }
            None => sforge::commands::session_tui::run(&config),
        },
        None => sforge::commands::request::run(&config, cli.request),
    }
}
