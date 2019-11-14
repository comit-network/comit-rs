use super::file::{AllowedForeignOrigins, Cors, Database, File, Network, Socket};
use crate::config::file::{Bitcoin, Ethereum};
use anyhow::Context;
use log::LevelFilter;
use reqwest::Url;
use std::path::Path;

/// This structs represents the settings as they are used through out the code.
///
/// An optional setting (represented in this struct as an `Option`) has semantic
/// meaning in cnd. Contrary to that, many configuration values are optional in
/// the config file but may be replaced by default values when the `Settings`
/// are created from a given `Config`.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub network: Network,
    pub http_api: HttpApi,
    pub database: Database,
    pub logging: Logging,
    pub bitcoin: Bitcoin,
    pub ethereum: Ethereum,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HttpApi {
    pub socket: Socket,
    pub cors: Cors,
}

#[derive(Clone, Debug, PartialEq, derivative::Derivative)]
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
            network,
            http_api: HttpApi {
                socket: http_api.socket,
                cors: http_api.cors.unwrap_or(Cors {
                    allowed_foreign_origins: AllowedForeignOrigins::None,
                }),
            },
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
            http_api: file::HttpApi {
                socket: Socket {
                    address: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                    port: 8000,
                },
                cors: None,
            },
            ..File::default()
        };

        let settings = Settings::from_config_file_and_defaults(config_file);

        assert_that(&settings)
            .map(|settings| &settings.http_api.cors)
            .is_equal_to(Cors {
                allowed_foreign_origins: AllowedForeignOrigins::None,
            })
    }
}
