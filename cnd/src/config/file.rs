use crate::{seed::Seed, std_ext::path::PrintablePath};
use config as config_rs;
use directories::ProjectDirs;
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
        // Linux: /home/<user>/.config/cnd/cnd.toml
        // Windows: C:\Users\<user>\AppData\Roaming\comit-network\cnd\config\cnd.toml
        // OSX: /Users/<user>/Library/Preferences/comit-network.cnd/cnd.toml
        if let Some(proj_dirs) = ProjectDirs::from("", "comit-network", "cnd") {
            let path = proj_dirs.config_dir();
            return Ok(Path::join(path, "cnd.toml"));
        }

        Err(config_rs::ConfigError::Message(
            "Could not generate configuration directory".to_string(),
        ))
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
