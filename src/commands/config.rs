use crate::cli::{ConfigCommand, ConfigSetArgs, DirectoryAddArgs, DirectoryRemoveArgs};
use crate::config::{Config, DirectoryEntry};
use crate::error::Result;
use rust_embed::RustEmbed;
use std::fs;
use std::io::{self};
use std::path::PathBuf;

/// Embeds the contents of the assets/ directory into the binary for access at runtime.
/// Used for providing a default config.yml if one does not exist on disk.
#[derive(RustEmbed)]
#[folder = "assets/"]
struct Asset;

/// Returns the default text editor command for this platform.
/// - Windows: notepad
/// - Linux/macOS: nano
#[cfg(windows)]
fn default_editor() -> String {
    "notepad".to_string()
}

#[cfg(not(windows))]
fn default_editor() -> String {
    "nano".to_string()
}

/// Opens the config file in the user's preferred editor.
/// Tries $EDITOR env var, or falls back to the platform default.
fn edit_config_in_editor(path: &PathBuf) -> io::Result<()> {
    // Try $EDITOR, otherwise platform default
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| default_editor());
    std::process::Command::new(editor).arg(path).status()?;
    Ok(())
}

/// Prints the current YAML config file contents to stdout.
fn handle_config_show(path: &PathBuf) -> Result<()> {
    let content = fs::read_to_string(&path)?;
    println!("{content}");
    Ok(())
}

/// Opens the config file in an editor for manual modification.
fn handle_config_edit(path: &PathBuf) -> Result<()> {
    edit_config_in_editor(&path)?;
    Ok(())
}

/// Updates one or more top-level config fields (endpoint, backet, part_size, local_directory_path).
/// Only updates fields provided as Some(value).
fn handle_config_set(args: &ConfigSetArgs, path: &PathBuf) -> Result<()> {
    let mut config: Config = {
        let s = fs::read_to_string(&path)?;
        serde_yaml::from_str(&s)?
    };

    if let Some(val) = &args.endpoint {
        config.endpoint = val.to_string();
    }
    if let Some(val) = &args.backet {
        config.backet = val.to_string();
    }
    if let Some(val) = &args.part_size {
        config.part_size = *val;
    }
    if let Some(val) = &args.local_directory_path {
        config.local_directory_path = val.to_string();
    }

    let s = serde_yaml::to_string(&config)?;
    fs::write(&path, s)?;
    println!("Config updated.");

    Ok(())
}

/// Adds a new directory mapping to the config's directory_struct.
/// Skips addition if the prefix_file already exists.
fn handle_config_dir_add(args: &DirectoryAddArgs, path: &PathBuf) -> Result<()> {
    let mut config: Config = {
        let s = fs::read_to_string(&path)?;
        serde_yaml::from_str(&s)?
    };

    if config
        .directory_struct
        .iter()
        .any(|entry| entry.prefix_file == args.prefix_file)
    {
        println!("Entry with this prefix_file already exists.");
    } else {
        config.directory_struct.push(DirectoryEntry {
            prefix_file: args.prefix_file.to_string(),
            cloud_dir: args.cloud_dir.to_string(),
        });

        let s = serde_yaml::to_string(&config)?;
        fs::write(&path, s)?;

        println!("Directory entry added.");
    }

    Ok(())
}

/// Removes a directory mapping from the config's directory_struct by prefix_file.
/// Notifies the user if no such entry was found.
fn handle_config_dir_rm(args: &DirectoryRemoveArgs, path: &PathBuf) -> Result<()> {
    let mut config: Config = {
        let s = fs::read_to_string(&path)?;
        serde_yaml::from_str(&s)?
    };

    let old_len = config.directory_struct.len();
    config
        .directory_struct
        .retain(|entry| entry.prefix_file != args.prefix_file);
    if config.directory_struct.len() < old_len {
        let s = serde_yaml::to_string(&config)?;
        fs::write(&path, s)?;

        println!("Directory entry removed.");
    } else {
        println!("No entry with such prefix_file found.");
    }

    Ok(())
}

/// Returns the full path to the platform-native config file.
/// - Linux/macOS: ~/.config/prefixload/config.yml
/// - Windows: %APPDATA%\prefixload\config.yml
fn config_path() -> PathBuf {
    let mut dir = dirs_next::config_dir().expect("Cannot get config directory");
    dir.push("prefixload");
    if !dir.exists() {
        fs::create_dir_all(&dir).expect("Cannot create config directory");
    }
    dir.push("config.yml");

    dir
}

/// Ensures that the config file exists at the standard path.
/// If not, writes the embedded default config.yml from the binary.
fn ensure_config_exists(path: &PathBuf) -> Result<()> {
    if !path.exists() {
        let bytes = Asset::get("config.yml")
            .expect("Embedded config.yaml not found")
            .data;
        std::fs::write(path, bytes)?;
        println!("Default config.yml written to {}", path.display());
    }
    Ok(())
}

/// Handles all config subcommands.
/// Ensures config file exists before dispatching to the relevant handler.
/// Returns an empty string as a placeholder output.
pub fn run(cmd: ConfigCommand) -> Result<String> {
    let path = config_path();
    ensure_config_exists(&path)?;

    match cmd {
        ConfigCommand::Show => handle_config_show(&path)?,
        ConfigCommand::Edit => handle_config_edit(&path)?,
        ConfigCommand::Set(args) => handle_config_set(&args, &path)?,
        ConfigCommand::DirAdd(args) => handle_config_dir_add(&args, &path)?,
        ConfigCommand::DirRm(args) => handle_config_dir_rm(&args, &path)?,
    }

    Ok("".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    // Helper: Returns a path in a temp dir for testing
    fn test_config_path(tmpdir: &tempfile::TempDir) -> PathBuf {
        tmpdir.path().join("config.yml")
    }

    fn write_embedded_default(path: &PathBuf) {
        let bytes = Asset::get("config.yml").expect("No embedded config").data;
        fs::write(path, bytes).unwrap();
    }

    #[test]
    fn test_ensure_config_exists_creates_file() {
        let dir = tempdir().unwrap();
        let path = test_config_path(&dir);

        // Should create config if it doesn't exist
        assert!(!path.exists());
        ensure_config_exists(&path).unwrap();
        assert!(path.exists());

        // Should not overwrite existing config
        fs::write(&path, b"test").unwrap();
        ensure_config_exists(&path).unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "test");
    }

    #[test]
    fn test_handle_config_set_updates_fields() {
        let dir = tempdir().unwrap();
        let path = test_config_path(&dir);
        write_embedded_default(&path);

        let args = ConfigSetArgs {
            endpoint: Some("custom-endpoint".into()),
            backet: None,
            part_size: Some(4242),
            local_directory_path: None,
        };

        handle_config_set(&args, &path).unwrap();

        let updated: Config = {
            let s = fs::read_to_string(&path).unwrap();
            serde_yaml::from_str(&s).unwrap()
        };

        assert_eq!(updated.endpoint, "custom-endpoint");
        assert_eq!(updated.part_size, 4242);
    }

    #[test]
    fn test_handle_config_dir_add_and_rm() {
        let dir = tempdir().unwrap();
        let path = test_config_path(&dir);
        write_embedded_default(&path);

        // Читаем начальный config
        let initial_config: Config = {
            let s = fs::read_to_string(&path).unwrap();
            serde_yaml::from_str(&s).unwrap()
        };
        let initial_len = initial_config.directory_struct.len();

        // Add entry
        let add_args = DirectoryAddArgs {
            prefix_file: "prefix_test".into(),
            cloud_dir: "cloud_dir_test".into(),
        };
        handle_config_dir_add(&add_args, &path).unwrap();

        let config: Config = {
            let s = fs::read_to_string(&path).unwrap();
            serde_yaml::from_str(&s).unwrap()
        };
        // There should be +1 to the initial amount
        assert_eq!(config.directory_struct.len(), initial_len + 1);
        assert!(
            config
                .directory_struct
                .iter()
                .any(|d| d.prefix_file == "prefix_test")
        );

        // Remove entry
        let rm_args = DirectoryRemoveArgs {
            prefix_file: "prefix_test".into(),
        };
        handle_config_dir_rm(&rm_args, &path).unwrap();
        let config: Config = {
            let s = fs::read_to_string(&path).unwrap();
            serde_yaml::from_str(&s).unwrap()
        };
        assert_eq!(config.directory_struct.len(), initial_len);
    }
}
