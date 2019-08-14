use crate::settings::Settings;
use config::ConfigError;
use directories::ProjectDirs;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Opt {
    /// Path to configuration folder
    #[structopt(short = "c", long = "config", parse(from_os_str))]
    config_file: Option<PathBuf>,
}

pub fn load_settings(opt: Opt) -> Result<Settings, ConfigError> {
    match opt.config_file {
        Some(file) => parse_config_file(file),
        None => match ProjectDirs::from("tech", "CoBloX", "btsieve") {
            // Linux: /home/<user>/.config/btsieve
            // Windows: C:\Users\<user>\AppData\Roaming\CoBloX\btsieve\config
            // OSX: /Users/<user>/Library/Preferences/tech.CoBloX.btsieve
            Some(proj_dirs) => {
                let file = Path::join(proj_dirs.config_dir(), "btsieve.toml");
                log::info!("Config file was not provided - looking up config file in default location at: {:?}", file);;
                parse_config_file(file)
            }
            None => Err(ConfigError::Message(
                "Could not generate configuration directory".to_string(),
            )),
        },
    }
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
