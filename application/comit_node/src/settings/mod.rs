mod serde_duration;
mod serde_log;

use crate::seed::Seed;
use config::{Config, ConfigError, File};
use libp2p::Multiaddr;
use log::LevelFilter;
use rand::Rng;
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
        let comit_listen = "/ip4/0.0.0.0/tcp/8011".parse().unwrap();
        let btsieve_url = Url::parse("http://localhost:8181").unwrap();
        let data = rand::thread_rng().gen::<[u8; 32]>();

        ComitNodeSettings {
            comit: Comit {
                secret_seed: Seed::from(data),
            },
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
    pub fn create_with_default(path: PathBuf, file_name: &str) -> Result<Self, ConfigError> {
        if !path.exists() {
            log::warn!(
                "Config path does not exist, creating folders recursively: {:?}",
                path
            );
            fs::create_dir_all(path.clone()).map_err(|error| {
                ConfigError::Message(format!("Could not create folders: {:?}: {:?}", path, error))
            })?;
        }

        let default_config = path.join(file_name);

        if default_config.exists() {
            if default_config.is_file() {
                log::warn!(
                    "Config files exists, loading config from: {:?}",
                    default_config
                );
                return Self::read(default_config);
            } else {
                return Err(ConfigError::Message(format!(
                    "Config file exists but is not a file: {:?}",
                    default_config
                )));
            }
        }

        let default_settings = ComitNodeSettings::default();
        let toml_string = toml::to_string(&default_settings).map_err(|error| {
            ConfigError::Message(format!("Could not serialize config: {:?}", error))
        })?;

        let mut file = std::fs::File::create(default_config.clone()).map_err(|error| {
            ConfigError::Message(format!(
                "Could not create config file: {:?} {:?}",
                default_config, error
            ))
        })?;

        file.write_all(toml_string.as_bytes()).map_err(|error| {
            ConfigError::Message(format!(
                "Could not write to file: {:?}: {:?}",
                default_config, error
            ))
        })?;
        Ok(default_settings)
    }

    pub fn read<D: AsRef<OsStr>>(default_config: D) -> Result<Self, ConfigError> {
        let mut config = Config::new();

        let default_config_file = Path::new(&default_config);

        // Start off by merging in the "default" configuration file
        config.merge(File::from(default_config_file))?;

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
        ComitNodeSettings::read("./config/default.toml")
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
        let config_file = "default.toml";

        delete_tmp_files(&config_path, config_file);

        let default_settings =
            ComitNodeSettings::create_with_default(config_path.clone(), config_file.clone());
        let settings = ComitNodeSettings::read(config_path.join(config_file));
        delete_tmp_files(&config_path, config_file);

        assert_that(&default_settings).is_ok();
        assert_that(&settings).is_ok();
        assert_that(&default_settings.unwrap()).is_equal_to(&settings.unwrap());

        delete_tmp_files(&config_path, config_file);
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
