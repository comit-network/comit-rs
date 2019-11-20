use super::file::{self, Database, File, Network, Socket};
use crate::config::file::{Bitcoin, Ethereum};
use anyhow::Context;
use log::LevelFilter;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::{
    net::{IpAddr, Ipv4Addr},
    path::Path,
};

/// This structs represents the settings as they are used through out the code.
///
/// An optional setting (represented in this struct as an `Option`) has semantic
/// meaning in cnd. Contrary to that, many configuration values are optional in
/// the config file but may be replaced by default values when the `Settings`
/// are created from a given `Config`.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Settings {
    pub network: Network,
    pub http_api: HttpApi,
    pub database: Database,
    pub logging: Logging,
    pub bitcoin: Bitcoin,
    pub ethereum: Ethereum,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct HttpApi {
    pub socket: Socket,
    pub cors: Cors,
}

impl Default for HttpApi {
    fn default() -> Self {
        Self {
            socket: Socket {
                address: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                port: 8000,
            },
            cors: Cors::default(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Cors {
    pub allowed_origins: AllowedOrigins,
}

impl Default for Cors {
    fn default() -> Self {
        Self {
            allowed_origins: AllowedOrigins::None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AllowedOrigins {
    All,
    None,
    Some(Vec<String>),
}

#[derive(Clone, Debug, Deserialize, PartialEq, derivative::Derivative, Serialize)]
#[derivative(Default)]
pub struct Logging {
    #[derivative(Default(value = "LevelFilter::Debug"))]
    pub level: LevelFilter,
    pub structured: bool,
}

impl Settings {
    pub fn from_config_file_and_defaults(config_file: File) -> anyhow::Result<Self> {
        let File {
            network,
            http_api,
            database,
            logging,
            bitcoin,
            ethereum,
        } = config_file;

        Ok(Self {
            network: network.unwrap_or_else(|| {
                let default_socket = "/ip4/0.0.0.0/tcp/9939"
                    .parse()
                    .expect("cnd listen address could not be parsed");

                Network {
                    listen: vec![default_socket],
                }
            }),
            http_api: http_api
                .map(|file::HttpApi { socket, cors }| {
                    let cors = cors
                        .map(|cors| {
                            let allowed_origins = match cors.allowed_origins {
                                file::AllowedOrigins::All(_) => AllowedOrigins::All,
                                file::AllowedOrigins::None(_) => AllowedOrigins::None,
                                file::AllowedOrigins::Some(origins) => {
                                    AllowedOrigins::Some(origins)
                                }
                            };

                            Cors { allowed_origins }
                        })
                        .unwrap_or_default();

                    HttpApi { socket, cors }
                })
                .unwrap_or_default(),
            database: {
                let default_database_path = crate::data_dir()
                    .map(|dir| Path::join(&dir, "cnd.sqlite"))
                    .context("unable to determine default database path")?;
                database.unwrap_or_else(|| Database {
                    sqlite: default_database_path,
                })
            },
            logging: {
                let Logging {
                    level: default_level,
                    structured: default_structured,
                } = Logging::default();
                logging
                    .map(|logging| Logging {
                        level: logging.level.unwrap_or(default_level),
                        structured: logging.structured.unwrap_or(default_structured),
                    })
                    .unwrap_or_default()
            },
            bitcoin: bitcoin.unwrap_or_else(|| Bitcoin {
                network: bitcoin::Network::Regtest,
                node_url: Url::parse("http://localhost:18443")
                    .expect("static string to be a valid url"),
            }),
            ethereum: ethereum.unwrap_or_else(|| Ethereum {
                node_url: Url::parse("http://localhost:8545")
                    .expect("static string to be a valid url"),
            }),
        })
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::config::file;
    use spectral::prelude::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn field_structured_defaults_to_false() {
        let config_file = File {
            logging: Some(file::Logging {
                level: None,
                structured: None,
            }),
            ..File::default()
        };

        let settings = Settings::from_config_file_and_defaults(config_file);

        assert_that(&settings)
            .is_ok()
            .map(|settings| &settings.logging.structured)
            .is_false()
    }

    #[test]
    fn field_structured_is_correctly_mapped() {
        let config_file = File {
            logging: Some(file::Logging {
                level: None,
                structured: Some(true),
            }),
            ..File::default()
        };

        let settings = Settings::from_config_file_and_defaults(config_file);

        assert_that(&settings)
            .is_ok()
            .map(|settings| &settings.logging.structured)
            .is_true()
    }

    #[test]
    fn logging_section_defaults_to_debug_and_false() {
        let config_file = File {
            logging: None,
            ..File::default()
        };

        let settings = Settings::from_config_file_and_defaults(config_file);

        assert_that(&settings)
            .is_ok()
            .map(|settings| &settings.logging)
            .is_equal_to(Logging {
                level: LevelFilter::Debug,
                structured: false,
            })
    }

    #[test]
    fn cors_section_defaults_to_no_allowed_foreign_origins() {
        let config_file = File {
            http_api: Some(file::HttpApi {
                socket: Socket {
                    address: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                    port: 8000,
                },
                cors: None,
            }),
            ..File::default()
        };

        let settings = Settings::from_config_file_and_defaults(config_file);

        assert_that(&settings)
            .is_ok()
            .map(|settings| &settings.http_api.cors)
            .is_equal_to(Cors {
                allowed_origins: AllowedOrigins::None,
            })
    }

    #[test]
    fn http_api_section_defaults() {
        let config_file = File {
            http_api: None,
            ..File::default()
        };

        let settings = Settings::from_config_file_and_defaults(config_file);

        assert_that(&settings)
            .is_ok()
            .map(|settings| &settings.http_api)
            .is_equal_to(HttpApi {
                socket: Socket {
                    address: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                    port: 8000,
                },
                cors: Cors {
                    allowed_origins: AllowedOrigins::None,
                },
            })
    }

    #[test]
    fn network_section_defaults() {
        let config_file = File {
            network: None,
            ..File::default()
        };

        let settings = Settings::from_config_file_and_defaults(config_file);

        assert_that(&settings)
            .is_ok()
            .map(|settings| &settings.network)
            .is_equal_to(Network {
                listen: vec!["/ip4/0.0.0.0/tcp/9939".parse().unwrap()],
            })
    }

    // Note toml library serializes Network without a trailing new
    #[test]
    fn serializes_correctly() {
        let file = File::default();
        let settings = Settings::from_config_file_and_defaults(file).unwrap();

        let want = r#"[network]
listen = ["/ip4/0.0.0.0/tcp/9939"]
[http_api.socket]
address = "0.0.0.0"
port = 8000

[http_api.cors]
allowed_origins = "none"

[database]
sqlite = "/home/tobin/.local/share/comit/cnd.sqlite"

[logging]
level = "DEBUG"
structured = false

[bitcoin]
network = "regtest"
node_url = "http://localhost:18443/"

[ethereum]
node_url = "http://localhost:8545/"
"#;
        let got = toml::to_string(&settings).unwrap();
        assert_that(&got).is_equal_to(String::from(want));
    }
}
