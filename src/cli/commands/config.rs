use crate::cli::{ConfigCommand, ConfigSetArgs, DirectoryAddArgs, DirectoryRemoveArgs};
use crate::config::{Config, DirectoryEntry};
use crate::error::{PrefixloadError, Result};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::{LinesWithEndings, as_24_bit_terminal_escaped};

/// Prints the current YAML config file contents to stdout with syntax highlighting.
/// Falls back to plain text if the syntax highlighting theme is not found.
fn handle_config_show() -> Result<String> {
    let content = Config::read_to_string()?;

    let ts = ThemeSet::load_defaults();

    // Try to get the theme; if it's not found, just return the raw content.
    if let Some(theme) = ts.themes.get("base16-ocean.dark") {
        let ps = SyntaxSet::load_defaults_newlines();
        let syntax = ps
            .find_syntax_by_extension("yml")
            .or_else(|| ps.find_syntax_by_extension("yaml"))
            .unwrap_or_else(|| ps.find_syntax_plain_text());

        let mut h = HighlightLines::new(syntax, theme);
        let mut buf = String::with_capacity(content.len() * 2);

        for line in LinesWithEndings::from(&content) {
            let ranges = h.highlight_line(line, &ps)?;
            let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
            buf.push_str(&escaped);
        }
        Ok(buf)
    } else {
        // Fallback to plain text if theme is missing
        Ok(content)
    }
}

/// Opens the config file in the user's preferred editor.
/// Tries $EDITOR env var, or falls back to the platform default.
fn handle_config_edit() -> Result<String> {
    Config::edit()?;
    Ok("".to_string())
}

/// A generic helper for updating the config file.
///
/// This function abstracts the common pattern of:
/// 1. Loading the `Config` from disk.
/// 2. Applying a mutation to it.
/// 3. Saving the modified `Config` back to disk.
///
/// It takes a closure `operation` that receives a mutable reference
/// to the loaded config and performs the desired changes.
fn update_config<F, R>(operation: F) -> Result<R>
where
    F: FnOnce(&mut Config) -> Result<R>,
{
    let mut config = Config::load()?;
    let result = operation(&mut config)?;
    config.save()?;
    Ok(result)
}

/// Updates one or more top-level config fields (endpoint, backet, part_size, local_directory_path).
/// Only updates fields provided as Some(value).
fn handle_config_set(args: &ConfigSetArgs) -> Result<String> {
    update_config(|config| {
        if let Some(val) = &args.endpoint {
            config.endpoint = val.clone();
        }
        if let Some(val) = &args.bucket {
            config.bucket = val.clone();
        }
        if let Some(val) = &args.region {
            config.region = val.clone();
        }
        if let Some(val) = args.force_path_style {
            config.force_path_style = val;
        }
        if let Some(val) = args.part_size {
            config.part_size = val;
        }
        if let Some(val) = &args.local_directory_path {
            config.local_directory_path = val.clone();
        }
        Ok("Config updated!".to_string())
    })
}

/// Adds a new directory mapping to the config's directory_struct.
/// Skips addition if the local_name_prefix already exists.
fn handle_config_dir_add(args: &DirectoryAddArgs) -> Result<String> {
    update_config(|config| {
        if config
            .directory_struct
            .iter()
            .any(|e| e.local_name_prefix == args.local_name_prefix)
        {
            return Err(PrefixloadError::Custom(
                "Entry with this local_name_prefix already exists.".to_string(),
            ));
        }
        config.directory_struct.push(DirectoryEntry {
            local_name_prefix: args.local_name_prefix.clone(),
            remote_path: args.remote_path.clone(),
        });
        Ok("Directory entry added.".to_string())
    })
}

/// Removes a directory mapping from the config's directory_struct by local_name_prefix.
/// Notifies the user if no such entry was found.
fn handle_config_dir_rm(args: &DirectoryRemoveArgs) -> Result<String> {
    update_config(|config| {
        let old_len = config.directory_struct.len();
        config
            .directory_struct
            .retain(|entry| entry.local_name_prefix != args.local_name_prefix);

        if config.directory_struct.len() < old_len {
            return Ok("Directory entry removed.".to_string());
        }

        Err(PrefixloadError::Custom(
            "No entry with such local_name_prefix found.".to_string(),
        ))
    })
}

/// Handles all config subcommands.
/// Ensures config file exists before dispatching to the relevant handler.
/// Returns an empty string as a placeholder output.
pub async fn run(cmd: ConfigCommand) -> Result<String> {
    match cmd {
        ConfigCommand::Show => handle_config_show(),
        ConfigCommand::Edit => handle_config_edit(),
        ConfigCommand::Set(args) => handle_config_set(&args),
        ConfigCommand::DirAdd(args) => handle_config_dir_add(&args),
        ConfigCommand::DirRm(args) => handle_config_dir_rm(&args),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;
    use std::path::PathBuf;
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
    // handle_config_show
    // ---------------------------------------------------------------------

    #[test]
    #[serial]
    fn config_show_returns_content() {
        let _guard = temp_config_dir();

        let result = handle_config_show().expect("handle_config_show should not fail");

        // DEBUG: Print the result to see what the test is getting.
        println!("Test result content: '{}'", &result);

        // Check that the result (with or without backlight) contains the keywords
        // lines from the source file. This confirms that the file has been read.
        assert!(result.contains("endpoint"));
        assert!(result.contains("bucket"));
        assert!(result.contains("directory_struct"));
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
            bucket: Some("mybucket".into()), // NB: field name in CLI struct
            region: Some("eu-central-1".into()),
            force_path_style: Some(true),
            part_size: Some(123),
            local_directory_path: Some("/tmp/data".into()),
        };

        let msg = handle_config_set(&args).expect("set");
        assert_eq!(msg, "Config updated!");

        let cfg = Config::load().unwrap();
        assert_eq!(cfg.endpoint, "http://example.com");
        assert_eq!(cfg.bucket, "mybucket");
        assert_eq!(cfg.region, "eu-central-1");
        assert_eq!(cfg.force_path_style, true);
        assert_eq!(cfg.part_size, 123);
        assert_eq!(cfg.local_directory_path, PathBuf::from("/tmp/data"));
    }

    // ---------------------------------------------------------------------
    // handle_config_dir_add
    // ---------------------------------------------------------------------

    #[test]
    #[serial]
    fn dir_add_appends_and_skips_duplicate() {
        let _guard = temp_config_dir();

        let add_args = DirectoryAddArgs {
            local_name_prefix: "PRE".into(),
            remote_path: "dir1/".into(),
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
                .any(|e| e.local_name_prefix == "PRE" && e.remote_path == "dir1/"),
            "New directory mapping not found in config"
        );

        // Second insertion with the same prefix should be rejected
        let err_msg = handle_config_dir_add(&add_args).unwrap_err().to_string();
        assert!(err_msg.contains("Entry with this local_name_prefix already exists."));

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
            local_name_prefix: "DEL".into(),
            remote_path: "to/delete".into(),
        };
        handle_config_dir_add(&add_args).unwrap();

        // Remove it
        let rm_args = DirectoryRemoveArgs {
            local_name_prefix: "DEL".into(),
        };
        let msg1 = handle_config_dir_rm(&rm_args).expect("dir rm 1");
        assert_eq!(msg1, "Directory entry removed.");

        let cfg = Config::load().unwrap();
        assert!(
            cfg.directory_struct
                .iter()
                .all(|e| e.local_name_prefix != "DEL"),
            "Entry was not actually removed"
        );

        // Second attempt should say it does not exist
        let msg_err = handle_config_dir_rm(&rm_args).unwrap_err().to_string();
        assert!(msg_err.contains("No entry with such local_name_prefix found."));
    }
}
