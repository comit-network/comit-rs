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
use url::Url;

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
    pub btsieve: Btsieve,
    pub web_gui: Option<HttpSocket>,
    pub logging: Option<Logging>,
}

impl File {
    pub fn default<R: Rng>(rand: R) -> Self {
        let comit_listen = "/ip4/0.0.0.0/tcp/9939"
            .parse()
            .expect("cnd listen address could not be parsed");
        let btsieve_url =
            Url::parse("http://localhost:8181").expect("Btsieve url could not be created");
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
            btsieve: Btsieve {
                url: btsieve_url,
                bitcoin: PollParameters {
                    poll_interval_secs: Duration::from_secs(300),
                    network: bitcoin_support::Network::Regtest,
                },
                ethereum: PollParameters {
                    poll_interval_secs: Duration::from_secs(20),
                    network: ethereum_support::Network::Regtest,
                },
            },
            web_gui: Some(HttpSocket {
                address: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                port: 8080,
            }),
            logging: None,
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
pub struct Btsieve {
    #[serde(with = "url_serde")]
    pub url: url::Url,
    pub bitcoin: PollParameters<bitcoin_support::Network>,
    pub ethereum: PollParameters<ethereum_support::Network>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct PollParameters<T> {
    #[serde(with = "super::serde_duration")]
    pub poll_interval_secs: Duration,
    pub network: T,
}

impl File {
    pub fn read_or_create_default<R: Rng>(
        home_dir: Option<&Path>,
        rand: R,
    ) -> Result<Self, config_rs::ConfigError> {
        let default_config_path = home_dir.map(|dir| Self::compute_default_path(dir)).ok_or_else(|| {
            eprintln!("Failed to determine home directory and hence could not infer default config file location. You can specify a config file with `--config`.");
            config_rs::ConfigError::Message(
                "Failed to determine home directory".to_owned(),
            )
        })?;

        if default_config_path.exists() {
            println!("Using config file {}", PrintablePath(&default_config_path));

            Self::read(default_config_path)
        } else {
            println!(
                "Creating config file at {} because it does not exist yet",
                PrintablePath(&default_config_path)
            );
            Self::create_default_at(default_config_path, rand)
        }
    }

    pub fn compute_default_path(parent: &Path) -> PathBuf {
        let user_path_components: PathBuf = [".config", "comit", "cnd.toml"].iter().collect();

        parent.join(user_path_components)
    }

    pub fn read<D: AsRef<OsStr>>(config_file: D) -> Result<Self, config_rs::ConfigError> {
        let config_file = Path::new(&config_file);

        let mut config = config_rs::Config::new();
        config.merge(config_rs::File::from(config_file))?;
        config.try_into()
    }

    fn create_default_at<R: Rng>(
        default_config_path: PathBuf,
        rand: R,
    ) -> Result<Self, config_rs::ConfigError> {
        Self::default(rand).write_to(default_config_path)
    }

    pub fn write_to(self, config_file: PathBuf) -> Result<Self, config_rs::ConfigError> {
        Self::ensure_directory_exists(&config_file)?;

        Self::write_to_file(config_file, &self)?;

        Ok(self)
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
                    fs::create_dir_all(path).map_err(|error| {
                        config_rs::ConfigError::Message(format!(
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

    fn write_to_file(
        config_file: PathBuf,
        default_settings: &File,
    ) -> Result<(), config_rs::ConfigError> {
        let toml_string = toml::to_string(&default_settings).map_err(|error| {
            config_rs::ConfigError::Message(format!("Could not serialize config: {:?}", error))
        })?;
        let mut file = std::fs::File::create(config_file.clone()).map_err(|error| {
            config_rs::ConfigError::Message(format!(
                "Could not create config file: {:?} {:?}",
                config_file, error
            ))
        })?;
        file.write_all(toml_string.as_bytes()).map_err(|error| {
            config_rs::ConfigError::Message(format!(
                "Could not write to file: {:?}: {:?}",
                config_file, error
            ))
        })
    }
}
