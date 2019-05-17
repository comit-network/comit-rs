use crate::settings::ComitNodeSettings;
use config::ConfigError;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Opt {
    /// Path to configuration folder
    #[structopt(short = "c", long = "config", parse(from_os_str))]
    config_path: Option<PathBuf>,
}

pub fn load_settings(opt: Opt) -> Result<ComitNodeSettings, ConfigError> {
    let config_path = match opt.config_path {
        Some(config_path) => validate_path(config_path),
        None => match directories::UserDirs::new() {
            None => Err(ConfigError::Message(
                "Unable to determine user's home directory".to_string(),
            )),
            Some(dirs) => Ok(Path::join(dirs.home_dir(), ".config/comit_node")),
        },
    }?;
    let run_mode_config = crate::var_or_default("RUN_MODE", "development".into());
    let default_config = Path::join(&config_path, "default");
    let run_mode_config = Path::join(&config_path, run_mode_config);
    let settings = ComitNodeSettings::create(default_config, run_mode_config)?;
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
