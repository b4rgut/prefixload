pub mod commands;

use crate::error::Result;
use clap::{Args, Parser, Subcommand};

/// Nested subcommands for the `config` command.
/// Allows you to show, edit, and modify specific config fields.
#[derive(Subcommand, Debug, PartialEq)]
pub enum ConfigCommand {
    /// Show current configuration file content
    Show,
    /// Open configuration file in the default system editor ($EDITOR)
    Edit,
    /// Update one or more top-level fields in the config
    Set(ConfigSetArgs),
    /// Add an entry to the 'directory_struct' array
    DirAdd(DirectoryAddArgs),
    /// Remove an entry from 'directory_struct' by prefix_file
    DirRm(DirectoryRemoveArgs),
}

/// Arguments for the 'config set' subcommand.
/// Each field corresponds to a top-level config property.
#[derive(Args, Debug, PartialEq)]
pub struct ConfigSetArgs {
    /// S3 endpoint URL (e.g., https://s3.amazonaws.com)
    #[arg(long)]
    pub endpoint: Option<String>,
    /// S3 bucket name to use for backups
    #[arg(long)]
    pub bucket: Option<String>,
    /// Size of each upload part in bytes (default: 5MB)
    #[arg(long)]
    pub part_size: Option<u64>,
    /// Local directory path to scan for files
    #[arg(long)]
    pub local_directory_path: Option<String>,
}

/// Arguments for the 'config directory-add' subcommand.
/// Adds a new directory mapping.
#[derive(Args, Debug, PartialEq)]
pub struct DirectoryAddArgs {
    pub prefix_file: String,
    pub cloud_dir: String,
}

/// Arguments for the 'config directory-remove' subcommand.
/// Removes an entry by its prefix_file.
#[derive(Args, Debug, PartialEq)]
pub struct DirectoryRemoveArgs {
    pub prefix_file: String,
}

/// Top-level application subcommands
#[derive(Subcommand, Debug, PartialEq)]
pub enum Commands {
    /// Configuration management entrypoint.
    /// This variant accepts nested config subcommands.
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    /// Store your S3 credentials securely
    Login,
    /// Run the main backup operation (with optional 'quiet' mode)
    Run {
        #[arg(short, long, default_value_t = false)]
        quiet: bool,
    },
    /// Schedule a backup job using a cron expression
    Schedule { cron: String },
}

/// Application entrypoint.
/// This struct represents the full CLI and top-level command parsing.
#[derive(Parser, Debug, PartialEq)]
#[command(name = "prefixload")]
#[command(about = "S3 cli backup by file name prefix")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

impl Cli {
    /// Dispatch the parsed CLI command to the appropriate handler.
    /// Returns a Result with a String (output message or error).
    pub async fn run(self) -> Result<String> {
        match self.command {
            Commands::Config { command } => commands::config::run(command).await,
            Commands::Login => commands::login::run().await,
            Commands::Run { quiet } => commands::run::run(quiet).await,
            Commands::Schedule { cron } => commands::schedule::run(cron.as_ref()).await,
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
