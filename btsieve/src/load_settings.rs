#![allow(clippy::print_stdout)] // We cannot use `log` before we have the config file

use crate::settings::Settings;
use config::ConfigError;
use directories::ProjectDirs;
use std::path::{Path, PathBuf};

pub fn load_settings(config_file: Option<PathBuf>) -> Result<Settings, ConfigError> {
    // Allow config_file to override the default configuration file.
    if let Some(config_file) = config_file {
        return parse_config_file(config_file);
    }

    let path = config_dir()
        .map(|dir| Path::join(&dir, "btsieve.toml"))
        .ok_or_else(|| {
            ConfigError::Message("Could not generate default configuration path".to_string())
        })?;

    println!(
        "Config file was not provided - looking up config file in default location at: {:?}",
        path
    );

    parse_config_file(path)
}

fn parse_config_file(file: PathBuf) -> Result<Settings, ConfigError> {
    if file.exists() {
        Settings::read(file)
    } else {
        Err(ConfigError::Message(format!(
            "Could not load config file: {:?}",
            file
        )))
    }
}

// Linux: /home/<user>/.config/comit/
// Windows: C:\Users\<user>\AppData\Roaming\comit\config\
// OSX: /Users/<user>/Library/Preferences/comit/
fn config_dir() -> Option<PathBuf> {
    ProjectDirs::from("", "", "comit").map(|proj_dirs| proj_dirs.config_dir().to_path_buf())
}
