use crate::config::{Bitcoin, Database, Ethereum, Network, Socket};
use log::LevelFilter;
use serde::Deserialize;
use std::time::Duration;

/// This struct aims to represent the configuration file as it appears on disk.
///
/// Most importantly, optional elements of the configuration file are
/// represented as `Option`s` here. This allows us to create a dedicated step
/// for filling in default values for absent configuration options.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct File {
    pub network: Option<Network>,
    pub http_api: Option<HttpApi>,
    pub database: Option<Database>,
    pub logging: Option<Logging>,
    pub bitcoin: Option<Bitcoin>,
    pub ethereum: Option<Ethereum>,
}

impl File {
    pub fn default() -> Self {
        File {
            network: Option::None,
            http_api: Option::None,
            database: Option::None,
            logging: Option::None,
            bitcoin: Option::None,
            ethereum: Option::None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Logging {
    pub level: Option<LevelFilter>,
    pub structured: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct HttpApi {
    pub socket: Socket,
    pub cors: Option<Cors>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Cors {
    pub allowed_origins: AllowedOrigins,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum AllowedOrigins {
    All(All),
    None(None),
    Some(Vec<String>),
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum All {
    All,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum None {
    None,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct PollParameters<T> {
    #[serde(with = "super::serde_duration")]
    pub poll_interval_secs: Duration,
    pub network: T,
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::LevelFilter;
    use reqwest::Url;
    use spectral::prelude::*;
    use std::net::Ipv4Addr;

    #[derive(serde::Deserialize, PartialEq, Debug)]
    struct LoggingOnlyConfig {
        logging: Logging,
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
                level: Option::Some(LevelFilter::Debug),
                structured: Option::None,
            },
        });
    }

    #[test]
    fn bitcoin_deserializes_correctly() {
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

    #[test]
    fn cors_deserializes_correctly() {
        let file_contents = vec![
            r#"
            allowed_origins = "all"
            "#,
            r#"
             allowed_origins = "none"
            "#,
            r#"
             allowed_origins = ["http://localhost:8000", "https://192.168.1.55:3000"]
            "#,
        ];

        let expected = vec![
            Cors {
                allowed_origins: AllowedOrigins::All(All::All),
            },
            Cors {
                allowed_origins: AllowedOrigins::None(None::None),
            },
            Cors {
                allowed_origins: AllowedOrigins::Some(vec![
                    String::from("http://localhost:8000"),
                    String::from("https://192.168.1.55:3000"),
                ]),
            },
        ];

        let actual = file_contents
            .into_iter()
            .map(toml::from_str)
            .collect::<Result<Vec<Cors>, toml::de::Error>>()
            .unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn network_deserializes_correctly() {
        let file_contents = vec![
            r#"
            listen = ["/ip4/0.0.0.0/tcp/9939"]
            "#,
            r#"
            listen = ["/ip4/0.0.0.0/tcp/9939", "/ip4/127.0.0.1/tcp/9939"]
            "#,
        ];

        let expected = vec![
            Network {
                listen: vec!["/ip4/0.0.0.0/tcp/9939".parse().unwrap()],
            },
            Network {
                listen: (vec![
                    "/ip4/0.0.0.0/tcp/9939".parse().unwrap(),
                    "/ip4/127.0.0.1/tcp/9939".parse().unwrap(),
                ]),
            },
        ];

        let actual = file_contents
            .into_iter()
            .map(toml::from_str)
            .collect::<Result<Vec<Network>, toml::de::Error>>()
            .unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn full_config_deserializes_correctly() {
        let contents = r#"
[network]
listen = ["/ip4/0.0.0.0/tcp/9939"]

[http_api.socket]
address = "127.0.0.1"
port = 8000

[http_api.cors]
allowed_origins = "all"

[database]
sqlite = "/tmp/foobar.sqlite"

[logging]
level = "DEBUG"
structured = false

[bitcoin]
network = "mainnet"
node_url = "http://example.com/"

[ethereum]
node_url = "http://example.com/"
"#;

        let file = File {
            network: Some(Network {
                listen: vec!["/ip4/0.0.0.0/tcp/9939".parse().unwrap()],
            }),
            http_api: Some(HttpApi {
                socket: Socket {
                    address: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                    port: 8000,
                },
                cors: Some(Cors {
                    allowed_origins: AllowedOrigins::All(All::All),
                }),
            }),
            database: Some(Database {
                sqlite: PathBuf::from("/tmp/foobar.sqlite"),
            }),
            logging: Some(Logging {
                level: Some(LevelFilter::Debug),
                structured: Some(false),
            }),
            bitcoin: Some(Bitcoin {
                network: bitcoin::Network::Bitcoin,
                node_url: "http://example.com".parse().unwrap(),
            }),
            ethereum: Some(Ethereum {
                node_url: "http://example.com".parse().unwrap(),
            }),
        };

        let config = toml::from_str::<File>(contents);
        assert_that(&config).is_ok().is_equal_to(file.clone());
    }
}
