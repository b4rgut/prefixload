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

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::{env, fs};
    use tempfile::TempDir;

    // Environment variable that `dirs_next::config_dir()` relies on
    #[cfg(windows)]
    const CONFIG_ENV: &str = "APPDATA";
    #[cfg(not(windows))]
    const CONFIG_ENV: &str = "XDG_CONFIG_HOME";

    /// Creates a temporary config directory and overrides the system location.
    fn temp_config_dir() -> TempDir {
        let tmp = TempDir::new().expect("temp dir");
        // As of newer Rust versions `set_var` is marked unsafe
        unsafe { env::set_var(CONFIG_ENV, tmp.path()) };
        tmp
    }

    /// Verifies that the path is formed correctly and the directory is created.
    #[test]
    #[serial] // prevents other tests from mutating env in parallel
    fn config_path_creates_directory() {
        let _guard = temp_config_dir();
        let path = Config::config_path().expect("config path");

        assert!(
            path.ends_with("prefixload/config.yml"),
            "Expected path .../prefixload/config.yml, got {:?}",
            path
        );
        assert!(
            path.parent().unwrap().exists(),
            "`prefixload` directory was not created"
        );
    }

    /// Ensures that a default embedded `config.yml` is written if the file is missing.
    #[test]
    #[serial]
    fn ensure_config_creates_default_file() {
        let _guard = temp_config_dir();
        let path = Config::config_path().unwrap();

        // 1) file should not exist yet
        assert!(!path.exists());

        // 2) create default
        Config::ensure_config_exists(&path).unwrap();
        assert!(path.exists(), "`config.yml` was not created");

        // 3) compare contents
        let disk = fs::read(&path).unwrap();
        let embedded: Vec<u8> = Asset::get("config.yml").unwrap().data.into_owned();
        assert_eq!(
            disk, embedded,
            "Disk contents differ from embedded config.yml"
        );
    }

    /// Checks that `load` reads and deserializes YAML correctly.
    #[test]
    #[serial]
    fn load_parses_yaml() {
        let _guard = temp_config_dir();
        let cfg = Config::load().expect("load config");

        assert!(
            !cfg.endpoint.is_empty(),
            "`endpoint` should be populated in default YAML"
        );
        assert!(cfg.part_size > 0, "`part_size` must be > 0");
        assert!(
            !cfg.directory_struct.is_empty(),
            "`directory_struct` should not be empty"
        );
    }

    /// Ensures `save` creates a `.bak` and writes new data to the main file.
    #[test]
    #[serial]
    fn save_creates_backup_and_writes_new_content() {
        let _guard = temp_config_dir();

        // start with default
        let mut cfg = Config::load().unwrap();
        let path = Config::config_path().unwrap();
        let old = fs::read_to_string(&path).unwrap();

        // change a field and save
        cfg.endpoint = "http://example.com".into();
        cfg.save().unwrap();

        // .bak exists and matches the old content
        let mut bak = path.clone();
        bak.set_extension("yml.bak");
        assert!(bak.exists(), "Backup file was not created");

        let bak_content = fs::read_to_string(&bak).unwrap();
        assert_eq!(bak_content, old, "Backup does not match original file");

        // main file now contains the new value
        let saved: Config = serde_yaml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(
            saved.endpoint, "http://example.com",
            "`save` did not write new endpoint value"
        );
    }
}
