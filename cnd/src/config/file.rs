use crate::{seed::Seed, std_ext::path::PrintablePath};
use config as config_rs;
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

/// This struct aims to represent the configuration file as it appears on disk.
///
/// Most importantly, optional elements of the configuration file are
/// represented as `Option`s` here. This allows us to create a dedicated step
/// for filling in default values for absent configuration options.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct File {
    pub comit: Comit,
    pub network: Network,
    pub http_api: HttpSocket,
    pub database: Option<Database>,
    pub web_gui: Option<HttpSocket>,
    pub logging: Option<Logging>,
    pub bitcoin: Option<Bitcoin>,
    pub ethereum: Option<Ethereum>,
}

impl File {
    pub fn default<R: Rng>(rand: R) -> Self {
        let comit_listen = "/ip4/0.0.0.0/tcp/9939"
            .parse()
            .expect("cnd listen address could not be parsed");
        let seed = Seed::new_random(rand).expect("Could not generate random seed");

        File {
            comit: Comit { secret_seed: seed },
            network: Network {
                listen: vec![comit_listen],
            },
            http_api: HttpSocket {
                address: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                port: 8000,
            },
            database: None,
            web_gui: Some(HttpSocket {
                address: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                port: 8080,
            }),
            logging: None,
            bitcoin: None,
            ethereum: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Logging {
    pub level: Option<LevelFilter>,
    pub structured: Option<bool>,
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
pub struct PollParameters<T> {
    #[serde(with = "super::serde_duration")]
    pub poll_interval_secs: Duration,
    pub network: T,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Database {
    pub sqlite: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Bitcoin {
    #[serde(with = "super::serde_bitcoin_network")]
    pub network: bitcoin::Network,
    #[serde(with = "url_serde")]
    pub node_url: reqwest::Url,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Ethereum {
    pub network: ethereum_support::Network,
    #[serde(with = "url_serde")]
    pub node_url: reqwest::Url,
}

impl File {
    pub fn read_or_create_default<R: Rng>(rand: R) -> Result<Self, config_rs::ConfigError> {
        let path = Self::default_config_path()?;

        if path.exists() {
            println!(
                "Found configuration file, reading from {}",
                PrintablePath(&path)
            );
            Self::read(path)
        } else {
            println!(
                "No configuration file found, creating default at {}",
                PrintablePath(&path)
            );
            Self::default(rand).write_to(path)
        }
    }

    pub fn read<D: AsRef<OsStr>>(config_file: D) -> Result<Self, config_rs::ConfigError> {
        let config_file = Path::new(&config_file);

        let mut config = config_rs::Config::new();
        config.merge(config_rs::File::from(config_file))?;
        config.try_into()
    }

    pub fn write_to(self, config_file: PathBuf) -> Result<Self, config_rs::ConfigError> {
        Self::ensure_directory_exists(&config_file)?;

        Self::write_to_file(config_file, &self)?;

        Ok(self)
    }

    fn default_config_path() -> Result<PathBuf, config_rs::ConfigError> {
        crate::config_dir()
            .map(|dir| Path::join(&dir, "cnd.toml"))
            .ok_or_else(|| {
                config_rs::ConfigError::Message(
                    "Could not generate default configuration path".to_string(),
                )
            })
    }

    fn ensure_directory_exists(config_file: &PathBuf) -> Result<(), config_rs::ConfigError> {
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
                    fs::create_dir_all(path).map_err(|e| {
                        config_rs::ConfigError::Message(format!(
                            "Could not create folders: {:?}: {:?}",
                            path, e
                        ))
                    })
                } else {
                    Ok(())
                }
            }
        }
    }

    fn write_to_file(
        config_file: PathBuf,
        default_settings: &File,
    ) -> Result<(), config_rs::ConfigError> {
        let toml_string = toml::to_string(&default_settings).map_err(|e| {
            config_rs::ConfigError::Message(format!("Could not serialize config: {:?}", e))
        })?;
        let mut file = std::fs::File::create(config_file.clone()).map_err(|e| {
            config_rs::ConfigError::Message(format!(
                "Could not create config file: {:?} {:?}",
                config_file, e
            ))
        })?;
        file.write_all(toml_string.as_bytes()).map_err(|e| {
            config_rs::ConfigError::Message(format!(
                "Could not write to file: {:?}: {:?}",
                config_file, e
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::LevelFilter;
    use rand::rngs::mock::StepRng;
    use reqwest::Url;
    use spectral::prelude::*;
    use tempfile::NamedTempFile;

    #[derive(serde::Deserialize, PartialEq, Debug)]
    struct LoggingOnlyConfig {
        logging: Logging,
    }

    fn rng() -> StepRng {
        StepRng::new(0, 0)
    }

    #[test]
    fn structured_logging_flag_in_logging_section_is_optional() {
        let file_contents = r#"
        [logging]
        level = "DEBUG"
        "#;

        let config_file = toml::from_str(file_contents);

        assert_that(&config_file).is_ok_containing(LoggingOnlyConfig {
            logging: Logging {
                level: Some(LevelFilter::Debug),
                structured: None,
            },
        });
    }

    #[test]
    fn bitcoin_serializes_correctly() {
        let file_contents = vec![
            r#"
            network = "mainnet"
            node_url = "http://example.com"
            "#,
            r#"
            network = "testnet"
            node_url = "http://example.com"
            "#,
            r#"
            network = "regtest"
            node_url = "http://example.com"
            "#,
        ];

        let expected = vec![
            Bitcoin {
                network: bitcoin::Network::Bitcoin,
                node_url: Url::parse("http://example.com").unwrap(),
            },
            Bitcoin {
                network: bitcoin::Network::Testnet,
                node_url: Url::parse("http://example.com").unwrap(),
            },
            Bitcoin {
                network: bitcoin::Network::Regtest,
                node_url: Url::parse("http://example.com").unwrap(),
            },
        ];

        let actual = file_contents
            .into_iter()
            .map(toml::from_str)
            .collect::<Result<Vec<Bitcoin>, toml::de::Error>>()
            .unwrap();

        assert_eq!(actual, expected);
    }

    fn temp_toml_file() -> NamedTempFile {
        tempfile::Builder::new().suffix(".toml").tempfile().unwrap()
    }

    #[test]
    fn complete_logging_section_is_optional() {
        let config_without_logging_section = File {
            logging: None,
            ..File::default(rng())
        };
        let temp_file = temp_toml_file();
        let temp_file_path = temp_file.into_temp_path().to_path_buf();
        config_without_logging_section
            .write_to(temp_file_path.clone())
            .unwrap();

        let config_file_contents = std::fs::read_to_string(temp_file_path.clone()).unwrap();
        assert!(
            !config_file_contents.contains("[logging]"),
            "written config file should not contain logging section"
        );

        let config_file = File::read(temp_file_path);
        assert_that(&config_file)
            .is_ok()
            .map(|c| &c.logging)
            .is_none();
    }

    #[test]
    fn read_and_write_config_work() {
        let config = File::default(rng());
        let temp_file = temp_toml_file();
        let path = temp_file.into_temp_path().to_path_buf();

        let expected = config.write_to(path.clone()).unwrap();
        let actual = File::read(path);

        assert_that(&actual).is_ok_containing(&expected);
    }
}
