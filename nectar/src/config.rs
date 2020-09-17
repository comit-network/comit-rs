pub mod file;
mod seed;
mod serde;
pub mod settings;
pub mod validation;

use crate::{bitcoin, ethereum::dai};
use ::serde::{Deserialize, Serialize};
use anyhow::anyhow;
use libp2p::Multiaddr;
use std::path::PathBuf;
use url::Url;

pub use self::{file::File, seed::Seed, settings::*};
use anyhow::Context;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Data {
    pub dir: PathBuf,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Network {
    pub listen: Vec<Multiaddr>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Bitcoind {
    pub node_url: Url,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct MaxSell {
    #[serde(default)]
    #[serde(with = "crate::config::serde::bitcoin_amount")]
    pub bitcoin: Option<bitcoin::Amount>,
    #[serde(default)]
    #[serde(with = "crate::config::serde::dai_amount")]
    pub dai: Option<dai::Amount>,
}

pub fn read_config<T>(config_file: &Option<PathBuf>, default_config_path: T) -> anyhow::Result<File>
where
    T: FnOnce() -> anyhow::Result<PathBuf>,
{
    let path = config_file
        .as_ref()
        .map(|path| {
            eprintln!("Using config file {}", path.display());
            path
        })
        .map_or_else(
            || {
                // try to load default config
                let default_path = default_config_path()?;

                if default_path.exists() {
                    eprintln!(
                        "Using config file at default path: {}",
                        default_path.display()
                    );
                    Ok(default_path)
                } else {
                    eprintln!("Config file default path is {}", default_path.display());
                    Err(anyhow!("internal error (unreachable)"))
                }
            },
            |path| Ok(path.to_path_buf()),
        )
        .ok();
    match path {
        Some(path) => File::read(&path)
            .with_context(|| format!("failed to read config file {}", path.display())),
        None => Ok(File::default()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{bitcoin, config::file::Level, ethereum::ChainId, Spread};
    use comit::ledger;
    use std::{fs, io::Write};

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
    fn sample_config_deserializes_correctly() {
        let expected = File {
            maker: Some(file::Maker {
                max_sell: Some(MaxSell {
                    bitcoin: Some(bitcoin::Amount::from_btc(0.1).unwrap()),
                    dai: Some(dai::Amount::from_dai_trunc(1000.0).unwrap()),
                }),
                spread: Some(Spread::new(500).unwrap()),
                maximum_possible_fee: Some(file::Fees {
                    bitcoin: Some(bitcoin::Amount::from_btc(0.00009275).unwrap()),
                }),
                kraken_api_host: Some("https://api.kraken.com".parse().unwrap()),
            }),
            network: Some(Network {
                listen: vec!["/ip4/0.0.0.0/tcp/9939".parse().unwrap()],
            }),
            data: Some(Data {
                dir: "/Users/froyer/Library/Application Support/nectar"
                    .parse()
                    .unwrap(),
            }),
            logging: Some(file::Logging {
                level: Some(Level::Info),
            }),
            bitcoin: Some(file::Bitcoin {
                network: ledger::Bitcoin::Regtest,
                bitcoind: Some(Bitcoind {
                    node_url: "http://localhost:18443/".parse().unwrap(),
                }),
            }),
            ethereum: Some(file::Ethereum {
                chain_id: ChainId::MAINNET,
                node_url: Some("http://localhost:8545/".parse().unwrap()),
                local_dai_contract_address: None,
            }),
        };

        let config = read_config(
            &Some(PathBuf::from("sample-config.toml")),
            || unreachable!(),
        )
        .unwrap();

        assert_eq!(config, expected);
    }

    #[test]
    fn read_config_uses_default_path() {
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let default_path = tmp_dir.path().join("config.toml");

        let mut file = fs::File::create(default_path.clone()).unwrap();
        file.write_all(b"[data]\ndir = \"/not/a/default/location/\"")
            .unwrap();

        let default_path_fn = || Ok(default_path);

        let config = read_config(&None, default_path_fn).unwrap();
        assert_eq!(
            config.data.unwrap().dir,
            PathBuf::from("/not/a/default/location/")
        )
    }

    #[test]
    fn read_config_returns_default_config_if_default_path_errors() {
        let default_path_fn = || Err(anyhow!("Some error"));

        let config = read_config(&None, default_path_fn).unwrap();
        assert_eq!(config, File {
            maker: None,
            network: None,
            data: None,
            logging: None,
            bitcoin: None,
            ethereum: None,
        },)
    }

    #[test]
    fn read_config_errors_if_passed_path_doesnt_exist() {
        let default_path_fn = || unreachable!();

        let config = read_config(
            &Some(PathBuf::from("/this/path/doesnt/exist")),
            default_path_fn,
        );
        assert!(config.is_err())
    }
}
