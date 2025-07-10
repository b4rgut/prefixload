use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

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
    pub backet: String,
    pub part_size: u64,
    pub local_directory_path: String,
    pub directory_struct: Vec<DirectoryEntry>,
}

// These are general functions for loading/saving configs
impl Config {
    pub fn load(path: &PathBuf) -> Result<Self> {
        let s = fs::read_to_string(path)?;
        Ok(serde_yaml::from_str(&s)?)
    }
    pub fn save(&self, path: &PathBuf) -> Result<()> {
        let s = serde_yaml::to_string(self)?;
        fs::write(path, s)?;
        Ok(())
    }
}
