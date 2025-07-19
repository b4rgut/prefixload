use crate::cli::{ConfigCommand, ConfigSetArgs, DirectoryAddArgs, DirectoryRemoveArgs};
use crate::config::{Config, DirectoryEntry};
use crate::error::Result;
use std::io::{self, Write};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::{LinesWithEndings, as_24_bit_terminal_escaped};

/// Prints the current YAML config file contents to stdout with syntax highlighting.
fn handle_config_show() -> Result<String> {
    let content = Config::read_to_string()?;

    // Load syntax definitions and themes
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    // Try to use "yaml" syntax, fallback to plain if not found
    let syntax = ps
        .find_syntax_by_extension("yml")
        .or_else(|| ps.find_syntax_by_extension("yaml"))
        .unwrap_or_else(|| ps.find_syntax_plain_text());

    let theme = &ts.themes["base16-ocean.dark"];
    let mut h = HighlightLines::new(syntax, theme);

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    for line in LinesWithEndings::from(&content) {
        let ranges = h.highlight_line(line, &ps).unwrap();
        let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
        write!(handle, "{}", escaped)?;
    }

    Ok("".to_string())
}

/// Opens the config file in the user's preferred editor.
/// Tries $EDITOR env var, or falls back to the platform default.
fn handle_config_edit() -> Result<String> {
    let _ = Config::edit();

    Ok("".to_string())
}

/// Updates one or more top-level config fields (endpoint, backet, part_size, local_directory_path).
/// Only updates fields provided as Some(value).
fn handle_config_set(args: &ConfigSetArgs) -> Result<String> {
    let mut config: Config = Config::load()?;

    if let Some(val) = &args.endpoint {
        config.endpoint = val.to_string();
    }
    if let Some(val) = &args.backet {
        config.bucket = val.to_string();
    }
    if let Some(val) = &args.part_size {
        config.part_size = *val;
    }
    if let Some(val) = &args.local_directory_path {
        config.local_directory_path = val.to_string();
    }

    config.save()?;

    Ok("Config updated!\n".to_string())
}

/// Adds a new directory mapping to the config's directory_struct.
/// Skips addition if the prefix_file already exists.
fn handle_config_dir_add(args: &DirectoryAddArgs) -> Result<String> {
    let mut config: Config = Config::load()?;

    if config
        .directory_struct
        .iter()
        .any(|entry| entry.prefix_file == args.prefix_file)
    {
        return Ok("Entry with this prefix_file already exists.".to_string());
    }

    config.directory_struct.push(DirectoryEntry {
        prefix_file: args.prefix_file.to_string(),
        cloud_dir: args.cloud_dir.to_string(),
    });

    config.save()?;

    Ok("Directory entry added.".to_string())
}

/// Removes a directory mapping from the config's directory_struct by prefix_file.
/// Notifies the user if no such entry was found.
fn handle_config_dir_rm(args: &DirectoryRemoveArgs) -> Result<String> {
    let mut config: Config = Config::load()?;

    let old_len = config.directory_struct.len();
    config
        .directory_struct
        .retain(|entry| entry.prefix_file != args.prefix_file);

    if config.directory_struct.len() < old_len {
        config.save()?;
        return Ok("Directory entry removed.".to_string());
    }

    Ok("No entry with such prefix_file found.".to_string())
}

/// Handles all config subcommands.
/// Ensures config file exists before dispatching to the relevant handler.
/// Returns an empty string as a placeholder output.
pub async fn run(cmd: ConfigCommand) -> Result<String> {
    match cmd {
        ConfigCommand::Show => Ok(handle_config_show()?),
        ConfigCommand::Edit => Ok(handle_config_edit()?),
        ConfigCommand::Set(args) => Ok(handle_config_set(&args)?),
        ConfigCommand::DirAdd(args) => Ok(handle_config_dir_add(&args)?),
        ConfigCommand::DirRm(args) => Ok(handle_config_dir_rm(&args)?),
    }
}
