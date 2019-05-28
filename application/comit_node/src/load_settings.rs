use crate::settings::ComitNodeSettings;
use config::ConfigError;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Opt {
    /// Path to configuration file
    #[structopt(short = "c", long = "config", parse(from_os_str))]
    config_file: Option<PathBuf>,
}

pub fn load_settings(opt: Opt) -> Result<ComitNodeSettings, ConfigError> {
    match opt.config_file {
        Some(config_file) => {
            if config_file.exists() {
                ComitNodeSettings::read(config_file)
            } else {
                Err(ConfigError::Message(format!(
                    "Could not load config file: {:?}",
                    config_file
                )))
            }
        }
        None => match directories::UserDirs::new() {
            None => Err(ConfigError::Message(
                "Unable to determine user's home directory".to_string(),
            )),
            Some(dirs) => {
                let config_file = Path::join(dirs.home_dir(), ".config/comit_node/comit_node.toml");
                if config_file.exists() {
                    ComitNodeSettings::read(config_file)
                } else {
                    log::info!("config file was neither provided nor found at default location, generating default config at: {:?}", config_file);
                    ComitNodeSettings::default().write_to(config_file)
                }
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use crate::load_settings::{load_settings, Opt};
    use spectral::prelude::*;

    #[test]
    fn can_find_config_path() {
        let opt = Opt {
            config_file: Some("./config/comit_node.toml".into()),
        };
        let result = load_settings(opt);
        assert_that(&result).is_ok();
    }

    #[test]
    fn cannot_find_config_file_should_return_error() {
        let opt = Opt {
            config_file: Some("./config/unknown.toml".into()),
        };
        let result = load_settings(opt);
        assert_that(&result).is_err();
    }

    #[test]
    fn no_config_provided_should_start_fine() {
        let opt = Opt { config_file: None };
        let result = load_settings(opt);
        assert_that(&result).is_ok();
    }
}
