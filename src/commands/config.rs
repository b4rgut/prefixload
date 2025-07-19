use crate::cli::{ConfigCommand, ConfigSetArgs, DirectoryAddArgs, DirectoryRemoveArgs};
use crate::config::{Config, DirectoryEntry};
use crate::error::Result;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::{LinesWithEndings, as_24_bit_terminal_escaped};

/// Prints the current YAML config file contents to stdout with syntax highlighting.
fn handle_config_show() -> Result<String> {
    let content = Config::read_to_string()?;

    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let syntax = ps
        .find_syntax_by_extension("yml")
        .or_else(|| ps.find_syntax_by_extension("yaml"))
        .unwrap_or_else(|| ps.find_syntax_plain_text());

    let theme = &ts.themes["base16-ocean.dark"];
    let mut h = HighlightLines::new(syntax, theme);
    let mut buf = String::with_capacity(content.len() * 2);

    for line in LinesWithEndings::from(&content) {
        let ranges = h.highlight_line(line, &ps)?;
        let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
        buf.push_str(&escaped);
    }

    Ok(buf)
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

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;
    use tempfile::TempDir;

    // Environment variable that `dirs_next::config_dir()` consults
    #[cfg(windows)]
    const CONFIG_ENV: &str = "APPDATA";
    #[cfg(not(windows))]
    const CONFIG_ENV: &str = "XDG_CONFIG_HOME";

    /// Creates a temporary config directory and points the OS-specific
    /// config env-var at it so all file IO is sandboxed.
    fn temp_config_dir() -> TempDir {
        let tmp = TempDir::new().expect("temp dir");
        // `set_var` became unsafe in recent toolchains
        unsafe { env::set_var(CONFIG_ENV, tmp.path()) };
        tmp
    }

    // ---------------------------------------------------------------------
    // handle_config_set
    // ---------------------------------------------------------------------

    #[test]
    #[serial]
    fn config_set_updates_requested_fields() {
        let _guard = temp_config_dir();

        let args = ConfigSetArgs {
            endpoint: Some("http://example.com".into()),
            backet: Some("mybucket".into()), // NB: field name in CLI struct
            part_size: Some(123),
            local_directory_path: Some("/tmp/data".into()),
        };

        let msg = handle_config_set(&args).expect("set");
        assert_eq!(msg, "Config updated!\n");

        let cfg = Config::load().unwrap();
        assert_eq!(cfg.endpoint, "http://example.com");
        assert_eq!(cfg.bucket, "mybucket");
        assert_eq!(cfg.part_size, 123);
        assert_eq!(cfg.local_directory_path, "/tmp/data");
    }

    // ---------------------------------------------------------------------
    // handle_config_dir_add
    // ---------------------------------------------------------------------

    #[test]
    #[serial]
    fn dir_add_appends_and_skips_duplicate() {
        let _guard = temp_config_dir();

        let add_args = DirectoryAddArgs {
            prefix_file: "PRE".into(),
            cloud_dir: "dir1/".into(),
        };

        // First insertion succeeds
        let msg1 = handle_config_dir_add(&add_args).expect("dir add 1");
        assert_eq!(msg1, "Directory entry added.");

        let cfg_after_first = Config::load().unwrap();
        let initial_len = cfg_after_first.directory_struct.len();
        assert!(
            cfg_after_first
                .directory_struct
                .iter()
                .any(|e| e.prefix_file == "PRE" && e.cloud_dir == "dir1/"),
            "New directory mapping not found in config"
        );

        // Second insertion with the same prefix should be rejected
        let msg2 = handle_config_dir_add(&add_args).expect("dir add 2");
        assert_eq!(msg2, "Entry with this prefix_file already exists.");

        let cfg_after_second = Config::load().unwrap();
        assert_eq!(
            cfg_after_second.directory_struct.len(),
            initial_len,
            "Duplicate entry unexpectedly modified directory_struct"
        );
    }

    // ---------------------------------------------------------------------
    // handle_config_dir_rm
    // ---------------------------------------------------------------------

    #[test]
    #[serial]
    fn dir_rm_removes_and_reports_missing() {
        let _guard = temp_config_dir();

        // Seed a mapping we can delete
        let add_args = DirectoryAddArgs {
            prefix_file: "DEL".into(),
            cloud_dir: "to/delete".into(),
        };
        handle_config_dir_add(&add_args).unwrap();

        // Remove it
        let rm_args = DirectoryRemoveArgs {
            prefix_file: "DEL".into(),
        };
        let msg1 = handle_config_dir_rm(&rm_args).expect("dir rm 1");
        assert_eq!(msg1, "Directory entry removed.");

        let cfg = Config::load().unwrap();
        assert!(
            cfg.directory_struct.iter().all(|e| e.prefix_file != "DEL"),
            "Entry was not actually removed"
        );

        // Second attempt should say it does not exist
        let msg2 = handle_config_dir_rm(&rm_args).expect("dir rm 2");
        assert_eq!(msg2, "No entry with such prefix_file found.");
    }
}
