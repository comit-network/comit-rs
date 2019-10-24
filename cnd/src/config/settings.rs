use super::file::{Comit, Database, File, HttpSocket, Network};
use crate::config::file::{Bitcoin, Ethereum};
use log::LevelFilter;
use reqwest::Url;

/// This structs represents the settings as they are used through out the code.
///
/// An optional setting (represented in this struct as an `Option`) has semantic
/// meaning in cnd. Contrary to that, many configuration values are optional in
/// the config file but may be replaced by default values when the `Settings`
/// are created from a given `Config`.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub comit: Comit,
    pub network: Network,
    pub http_api: HttpSocket,
    pub database: Option<Database>,
    pub web_gui: Option<HttpSocket>,
    pub logging: Logging,
    pub bitcoin: Bitcoin,
    pub ethereum: Ethereum,
}

#[derive(Clone, Debug, PartialEq, derivative::Derivative)]
#[derivative(Default)]
pub struct Logging {
    #[derivative(Default(value = "LevelFilter::Debug"))]
    pub level: LevelFilter,
    pub structured: bool,
}

impl Settings {
    pub fn from_config_file_and_defaults(config_file: File) -> Self {
        let File {
            comit,
            network,
            http_api,
            database,
            web_gui,
            logging,
            bitcoin,
            ethereum,
        } = config_file;

        Self {
            comit,
            network,
            http_api,
            database,
            web_gui,
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
                network: ethereum_support::Network::Regtest,
                node_url: Url::parse("http://localhost:8545")
                    .expect("static string to be a valid url"),
            }),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::config::file;
    use rand::rngs::OsRng;
    use spectral::prelude::*;

    #[test]
    fn field_structured_defaults_to_false() {
        let config_file = File {
            logging: Some(file::Logging {
                level: None,
                structured: None,
            }),
            ..File::default(OsRng)
        };

        let settings = Settings::from_config_file_and_defaults(config_file);

        assert_that(&settings)
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
            ..File::default(OsRng)
        };

        let settings = Settings::from_config_file_and_defaults(config_file);

        assert_that(&settings)
            .map(|settings| &settings.logging.structured)
            .is_true()
    }

    #[test]
    fn logging_section_defaults_to_debug_and_false() {
        let config_file = File {
            logging: None,
            ..File::default(OsRng)
        };

        let settings = Settings::from_config_file_and_defaults(config_file);

        assert_that(&settings)
            .map(|settings| &settings.logging)
            .is_equal_to(Logging {
                level: LevelFilter::Debug,
                structured: false,
            })
    }
}
