use crate::settings::Settings;
use config::ConfigError;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Opt {
    /// Path to configuration folder
    #[structopt(short = "c", long = "config", parse(from_os_str))]
    config_path: Option<PathBuf>,
}

pub fn load_settings(opt: Opt) -> Result<Settings, ConfigError> {
    let config_path = match opt.config_path {
        Some(config_path) => validate_path(config_path),
        None => match directories::UserDirs::new() {
            None => Err(ConfigError::Message(
                "Unable to determine user's home directory".to_string(),
            )),
            Some(dirs) => Ok(Path::join(dirs.home_dir(), ".config/btsieve")),
        },
    }?;
    let default_config = Path::join(&config_path, "default");
    let settings = Settings::create(default_config)?;
    Ok(settings)
}

fn validate_path(path: PathBuf) -> Result<PathBuf, ConfigError> {
    match std::fs::metadata(path.clone()) {
        Ok(metadata) => {
            if metadata.is_dir() {
                Ok(path)
            } else {
                Err(ConfigError::Message(format!(
                    "Config path is expected to be a directory: {:?}",
                    path
                )))
            }
        }
        Err(e) => Err(ConfigError::Message(format!(
            "Cannot access config path {:?}: {:?}",
            path, e
        ))),
    }
}
