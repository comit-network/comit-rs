use anyhow::Context;
use std::path::{Path, PathBuf};

/// This is to store the configuration and seed files
// Linux: /home/<user>/.config/nectar/
// OSX: /Users/<user>/Library/Preferences/nectar/
fn config_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "nectar")
        .map(|proj_dirs| proj_dirs.config_dir().to_path_buf())
}

pub fn default_config_path() -> anyhow::Result<PathBuf> {
    config_dir()
        .map(|dir| Path::join(&dir, "config.toml"))
        .context("Could not generate default configuration path")
}

/// This is to store the DB
// Linux: /home/<user>/.local/share/nectar/
// OSX: /Users/<user>/Library/Application Support/nectar/
pub fn data_dir() -> Option<std::path::PathBuf> {
    directories::ProjectDirs::from("", "", "nectar")
        .map(|proj_dirs| proj_dirs.data_dir().to_path_buf())
}

pub fn ensure_directory_exists(file: &Path) -> Result<(), std::io::Error> {
    if let Some(path) = file.parent() {
        if !path.exists() {
            tracing::info!(
                "Parent directory does not exist, creating recursively: {}",
                file.display()
            );
            return std::fs::create_dir_all(path);
        }
    }
    Ok(())
}
