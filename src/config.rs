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

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MaxSell {
    #[serde(default)]
    #[serde(with = "crate::config::serde::bitcoin_amount")]
    pub bitcoin: Option<bitcoin::Amount>,
    #[serde(default)]
    #[serde(with = "crate::config::serde::dai_amount")]
    pub dai: Option<dai::Amount>,
}

pub fn read_config(config_file: &Option<PathBuf>) -> anyhow::Result<File> {
    let path = config_file
        .as_ref()
        .map(|path| {
            eprintln!("Using config file {}", path.display());
            path
        })
        .ok_or_else(|| {
            // try to load default config
            let default_path = crate::fs::default_config_path()?;

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
        })
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
}
