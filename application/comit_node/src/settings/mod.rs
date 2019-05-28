mod serde_duration;
mod serde_log;

use crate::seed::Seed;
use config::{Config, ConfigError, File};
use libp2p::Multiaddr;
use log::LevelFilter;
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    fs,
    io::Write,
    net::{IpAddr, Ipv4Addr},
    path::{Path, PathBuf},
    time::Duration,
};
use url::Url;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ComitNodeSettings {
    pub comit: Comit,
    pub network: Network,
    pub http_api: HttpSocket,
    pub btsieve: Btsieve,
    pub web_gui: Option<HttpSocket>,
    #[serde(default = "default_log_levels")]
    pub log_levels: LogLevels,
}

impl Default for ComitNodeSettings {
    fn default() -> Self {
        let comit_listen = "/ip4/0.0.0.0/tcp/8011"
            .parse()
            .expect("comit_node listen address could not be parsed");
        let btsieve_url =
            Url::parse("http://localhost:8181").expect("Btsieve url could not be created");
        let seed = Seed::new_random().expect("Could not generate random seed");

        ComitNodeSettings {
            comit: Comit { secret_seed: seed },
            network: Network {
                listen: vec![comit_listen],
            },
            http_api: HttpSocket {
                address: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                port: 8000,
            },
            btsieve: Btsieve {
                url: btsieve_url,
                bitcoin: PollParameters {
                    poll_interval_secs: Duration::from_secs(300),
                    network: "regtest".into(),
                },
                ethereum: PollParameters {
                    poll_interval_secs: Duration::from_secs(20),
                    network: "regtest".into(),
                },
            },
            web_gui: Some(HttpSocket {
                address: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                port: 8080,
            }),
            log_levels: LogLevels {
                comit_node: LevelFilter::Debug,
            },
        }
    }
}

fn default_log() -> LevelFilter {
    LevelFilter::Debug
}

fn default_log_levels() -> LogLevels {
    LogLevels {
        comit_node: default_log(),
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct LogLevels {
    #[serde(with = "self::serde_log", default = "default_log")]
    pub comit_node: LevelFilter,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Comit {
    pub secret_seed: Seed,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Network {
    pub listen: Vec<Multiaddr>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct HttpSocket {
    pub address: IpAddr,
    pub port: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Btsieve {
    #[serde(with = "url_serde")]
    pub url: url::Url,
    pub bitcoin: PollParameters,
    pub ethereum: PollParameters,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct PollParameters {
    #[serde(with = "self::serde_duration")]
    pub poll_interval_secs: Duration,
    pub network: String,
}

impl ComitNodeSettings {
    pub fn write_to(self, config_file: PathBuf) -> Result<Self, ConfigError> {
        ComitNodeSettings::ensure_directory_exists(&config_file)?;

        ComitNodeSettings::write_to_file(config_file, &self)?;

        Ok(self)
    }

    fn write_to_file(
        config_file: PathBuf,
        default_settings: &ComitNodeSettings,
    ) -> Result<(), ConfigError> {
        let toml_string = toml::to_string(&default_settings).map_err(|error| {
            ConfigError::Message(format!("Could not serialize config: {:?}", error))
        })?;
        let mut file = std::fs::File::create(config_file.clone()).map_err(|error| {
            ConfigError::Message(format!(
                "Could not create config file: {:?} {:?}",
                config_file, error
            ))
        })?;
        file.write_all(toml_string.as_bytes()).map_err(|error| {
            ConfigError::Message(format!(
                "Could not write to file: {:?}: {:?}",
                config_file, error
            ))
        })
    }

    fn ensure_directory_exists(config_file: &PathBuf) -> Result<(), ConfigError> {
        match config_file.parent() {
            None => {
                log::trace!("Config path is root path");
                Ok(())
            }
            Some(path) => {
                if !path.exists() {
                    log::debug!(
                        "Config path does not exist, creating directories recursively: {:?}",
                        path
                    );
                    fs::create_dir_all(path).map_err(|error| {
                        ConfigError::Message(format!(
                            "Could not create folders: {:?}: {:?}",
                            path, error
                        ))
                    })
                } else {
                    Ok(())
                }
            }
        }
    }

    pub fn read<D: AsRef<OsStr>>(config_file: D) -> Result<Self, ConfigError> {
        let mut config = Config::new();

        let config_file = Path::new(&config_file);

        // Start off by merging in the "default" configuration file
        config.merge(File::from(config_file))?;

        // You can deserialize (and thus freeze) the entire configuration as
        config.try_into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectral::prelude::*;
    use std::{env, fs};

    fn comit_settings() -> Result<ComitNodeSettings, ConfigError> {
        ComitNodeSettings::read("./config/comit_node.toml")
    }

    #[test]
    fn can_read_default_config() {
        let settings = comit_settings();

        assert_that(&settings).is_ok();
    }

    #[test]
    fn can_read_nested_parameters() {
        let settings = comit_settings();

        assert_that(&settings).is_ok();
        assert_that(&settings.unwrap().btsieve.ethereum.poll_interval_secs)
            .is_equal_to(&Duration::from_secs(20));
    }

    #[test]
    fn config_folder_does_not_exist_will_create_folder_and_config_file() {
        let tmp_dir = env::temp_dir();
        let config_path = Path::join(&tmp_dir, "i_am_invincible");
        let config_file = "comit_node.toml";

        delete_tmp_files(&config_path, config_file);

        let config_file_incl_path = config_path.clone().join(config_file.clone());

        let default_settings = ComitNodeSettings::default();

        let default_settings = default_settings.write_to(config_file_incl_path.clone());
        let settings = ComitNodeSettings::read(config_file_incl_path.clone());

        delete_tmp_files(&config_path, &config_file);

        let default_settings = assert_that(&default_settings).is_ok().subject;
        let settings = assert_that(&settings).is_ok().subject;
        assert_that(default_settings).is_equal_to(settings);
    }

    fn delete_tmp_files(config_path: &PathBuf, config_file: &str) {
        if config_path.exists() {
            if config_path.clone().join(config_file).exists() {
                let default_config_file = config_path.clone().join(config_file);
                fs::remove_file(default_config_file).unwrap();
            }
            fs::remove_dir(config_path.clone()).unwrap();
        }
    }
}
