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
    io::Write,
    net::{IpAddr, Ipv4Addr},
    path::{Path, PathBuf},
    time::Duration,
};
use url::Url;

#[derive(Debug, Deserialize, Serialize, PartialEq)]
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
        let listen = "/ip4/0.0.0.0/tcp/8011".parse().unwrap();
        let url = Url::parse("http://localhost:8001").unwrap();
        let data = rand::thread_rng().gen::<[u8; 32]>();

        ComitNodeSettings {
            comit: Comit {
                secret_seed: Seed::from(data),
            },
            network: Network {
                listen: vec![listen],
            },
            http_api: HttpSocket {
                address: IpAddr::V4(Ipv4Addr::LOCALHOST),
                port: 8000,
            },
            btsieve: Btsieve {
                url,
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
                address: IpAddr::V4(Ipv4Addr::LOCALHOST),
                port: 8080,
            }),
            log_levels: LogLevels {
                comit_node: LevelFilter::Debug,
            },
        }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct LogLevels {
    #[serde(with = "self::serde_log", default = "default_log")]
    pub comit_node: LevelFilter,
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
    pub fn create_with_default(default_config: PathBuf) -> Result<Self, ConfigError> {
        match std::fs::metadata(default_config.clone()) {
            Ok(metadata) => {
                if metadata.is_file() {
                    log::warn!(
                        "Config files exists, loading config from: {:?}",
                        default_config
                    );
                    Self::read(default_config)
                } else {
                    Err(ConfigError::Message(format!(
                        "Cannot create default config at: {:?}",
                        default_config
                    )))
                }
            }

            Err(_) => {
                let defaults = ComitNodeSettings::default();
                let toml_string = toml::to_string(&defaults).unwrap();

                let mut file = std::fs::File::create(default_config.clone()).unwrap();
                file.write_all(toml_string.as_bytes()).map_err(|error| {
                    ConfigError::Message(format!(
                        "Could not write to file: {:?}: {:?}",
                        default_config, error
                    ))
                })?;
                Ok(defaults)
            }
        }
    }

    pub fn read<D: AsRef<OsStr>>(default_config: D) -> Result<Self, ConfigError> {
        let mut config = Config::new();

        let default_config_file = Path::new(&default_config);

        // Start off by merging in the "default" configuration file
        config.merge(File::from(default_config_file))?;

        // Add in a local configuration file
        // This file shouldn't be checked in to git
        config.merge(File::with_name("config/local").required(false))?;

        // You can deserialize (and thus freeze) the entire configuration as
        config.try_into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::swap_protocols::rfc003::SecretSource;
    use spectral::prelude::*;
    use std::{
        env,
        fs::{self, File},
    };

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
    fn can_create_default_settings() {
        let mut tmp_dir = env::temp_dir();

        let config_path = Path::join(&tmp_dir, "default.toml");
        match std::fs::metadata(config_path.clone()) {
            Ok(metadata) => {
                if metadata.is_file() {
                    fs::remove_file(config_path.clone()).unwrap();
                }
            }
            Err(_) => (),
        };

        let default_settings = ComitNodeSettings::create_with_default(config_path.clone());
        let settings = ComitNodeSettings::read(config_path);

        assert_that(&default_settings).is_ok();
        assert_that(&settings).is_ok();

        assert_that(&default_settings.unwrap()).is_equal_to(&settings.unwrap());
    }
}
