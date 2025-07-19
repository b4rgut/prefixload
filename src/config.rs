use crate::error::Result;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Embeds the contents of the assets/ directory into the binary for access at runtime.
/// Used for providing a default config.yml if one does not exist on disk.
#[derive(RustEmbed)]
#[folder = "assets/"]
struct Asset;

/// Represents a mapping from a file prefix to a cloud directory.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DirectoryEntry {
    pub prefix_file: String,
    pub cloud_dir: String,
}

/// Represents the application's YAML configuration file.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub endpoint: String,
    pub bucket: String,
    pub part_size: u64,
    pub local_directory_path: String,
    pub directory_struct: Vec<DirectoryEntry>,
}

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

// These are general functions for loading/saving configs
impl Config {
    /// Returns the full path to the platform-native config file.
    /// - Linux/macOS: ~/.config/prefixload/config.yml
    /// - Windows: %APPDATA%\prefixload\config.yml
    pub fn config_path() -> Result<PathBuf> {
        let mut dir = dirs_next::config_dir().expect("Failed to get config directory");

        dir.push("prefixload");
        fs::create_dir_all(&dir)?;

        dir.push("config.yml");

        Ok(dir)
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

    pub fn read_to_string() -> Result<String> {
        let path = Self::config_path().unwrap();
        Self::ensure_config_exists(&path)?;

        let content = fs::read_to_string(&path)?;

        Ok(content)
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path().unwrap();
        Self::ensure_config_exists(&path)?;

        let s = fs::read_to_string(&path)?;

        Ok(serde_yaml::from_str(&s)?)
    }

    pub fn edit() -> Result<()> {
        let path = Self::config_path().unwrap();
        Self::backup_config()?;

        // Try $EDITOR, otherwise platform default
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| default_editor());
        std::process::Command::new(editor).arg(path).status()?;

        Ok(())
    }

    /// Creates or updates a backup of the config file as `config.yml.bak` before making changes.
    fn backup_config() -> Result<()> {
        let path = Self::config_path().unwrap();

        if path.exists() {
            let mut backup_path = path.clone();
            backup_path.set_extension("yml.bak"); // config.yml.bak
            std::fs::copy(path, &backup_path)?;
        }

        Ok(())
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path().unwrap();
        Self::backup_config()?;

        let s = serde_yaml::to_string(self)?;
        fs::write(path, s)?;

        Ok(())
    }
}
