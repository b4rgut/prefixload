use crate::commands;
use crate::error::Result;
use clap::{Parser, Subcommand};

#[derive(Subcommand, Debug, PartialEq)]
pub enum Commands {
    Login {
        access_key: String,
    },

    Run {
        #[arg(short, long, default_value_t = false)]
        quiet: bool,
    },

    Schedule {
        cron: String,
    },
}

#[derive(Parser, Debug, PartialEq)]
#[command(name = "prefixload")]
#[command(about = "TODO ...")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

impl Cli {
    pub fn run(self) -> Result<String> {
        match self.command {
            Commands::Login { access_key } => commands::login::run(access_key.as_ref()),
            Commands::Run { quiet } => commands::run::run(quiet),
            Commands::Schedule { cron } => commands::schedule::run(cron.as_ref()),
        }
    }

    /// Returns a reference to the parsed command.
    ///
    /// This method provides access to the command that was parsed from
    /// the command-line arguments. Useful for testing and introspection.
    pub fn get_command(&self) -> &Commands {
        &self.command
    }
}
