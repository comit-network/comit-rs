mod file;
mod settings;
mod validation;

use crate::{ethereum, ethereum::ChainId, fs};
use anyhow::{Context, Result};
use comit::ledger;
use conquer_once::Lazy;
use libp2p::Multiaddr;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, path::PathBuf, str::FromStr};

pub use self::{
    file::File,
    settings::{AllowedOrigins, Bitcoin, BitcoinFees, Settings},
    validation::validate_connection_to_network,
};

static BITCOIND_RPC_MAINNET: Lazy<Url> = Lazy::new(|| parse_unchecked("http://localhost:8332"));
static BITCOIND_RPC_TESTNET: Lazy<Url> = Lazy::new(|| parse_unchecked("http://localhost:18332"));
static BITCOIND_RPC_REGTEST: Lazy<Url> = Lazy::new(|| parse_unchecked("http://localhost:18443"));

static WEB3_URL: Lazy<Url> = Lazy::new(|| parse_unchecked("http://localhost:8545"));

/// The DAI token contract on Ethereum mainnet.
///
/// Source: https://changelog.makerdao.com/
static DAI_MAINNET: Lazy<ethereum::Address> =
    Lazy::new(|| parse_unchecked("0x6B175474E89094C44Da98b954EedeAC495271d0F"));

/// The DAI token contract on the Ethereum testnet "kovan".
///
/// Source: https://changelog.makerdao.com/
static DAI_KOVAN: Lazy<ethereum::Address> =
    Lazy::new(|| parse_unchecked("0x4F96Fe3b7A6Cf9725f59d353F723c1bDb64CA6Aa"));

/// The DAI token contract on the Ethereum testnet "ropsten".
///
/// Source: https://changelog.makerdao.com/
static DAI_ROPSTEN: Lazy<ethereum::Address> =
    Lazy::new(|| parse_unchecked("0x31F42841c2db5173425b5223809CF3A38FEde360"));

static COMIT_SOCKET: Lazy<Multiaddr> = Lazy::new(|| parse_unchecked("/ip4/0.0.0.0/tcp/9939"));

// Low value that would allow inclusion in ~6 blocks:
// https://txstats.com/dashboard/db/fee-estimation?orgId=1&panelId=2&fullscreen&from=now-6M&to=now&var-source=blockcypher
static FEERATE_SAT_PER_VBYTE: Lazy<bitcoin::Amount> = Lazy::new(|| bitcoin::Amount::from_sat(50));

static CYPHERBLOCK_MAINNET_URL: Lazy<Url> = Lazy::new(|| {
    "http://api.blockcypher.com/v1/btc/main"
        .parse()
        .expect("valid url")
});

static CYPHERBLOCK_TESTNET_URL: Lazy<Url> = Lazy::new(|| {
    "http://api.blockcypher.com/v1/btc/test3"
        .parse()
        .expect("valid url")
});

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Data {
    pub dir: PathBuf,
}

impl Data {
    pub fn default() -> Result<Self> {
        Ok(Self {
            dir: fs::data_dir().context("unable to determine default data path")?,
        })
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Bitcoind {
    pub node_url: Url,
}

impl Bitcoind {
    fn new(network: ledger::Bitcoin) -> Self {
        let node_url = match network {
            ledger::Bitcoin::Mainnet => BITCOIND_RPC_MAINNET.clone(),
            ledger::Bitcoin::Testnet => BITCOIND_RPC_TESTNET.clone(),
            ledger::Bitcoin::Regtest => BITCOIND_RPC_REGTEST.clone(),
        };

        Bitcoind { node_url }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Ethereum {
    pub chain_id: ChainId,
    pub geth: Geth,
    pub tokens: Tokens,
}

impl Ethereum {
    fn new(chain_id: ChainId) -> Result<Ethereum> {
        Ok(Self {
            chain_id,
            geth: Geth::new(),
            tokens: Tokens::new(chain_id)?,
        })
    }

    fn from_file(ethereum: file::Ethereum, comit_network: Option<comit::Network>) -> Result<Self> {
        if let Some(comit_network) = comit_network {
            let inferred = ChainId::from(comit_network);
            if inferred != ethereum.chain_id {
                anyhow::bail!(
                    "inferred Ethereum chain ID {} from CLI argument {} but config file says {}",
                    inferred,
                    comit_network,
                    ethereum.chain_id
                );
            }
        }

        let chain_id = ethereum.chain_id;
        let geth = ethereum.geth.unwrap_or_else(Geth::new);
        let tokens = ethereum.tokens.map_or_else(
            || Tokens::new(chain_id),
            |file| Tokens::from_file(file, chain_id),
        )?;

        Ok(Ethereum {
            chain_id,
            geth,
            tokens,
        })
    }
}

impl From<Ethereum> for file::Ethereum {
    fn from(ethereum: Ethereum) -> Self {
        file::Ethereum {
            chain_id: ethereum.chain_id,
            geth: Some(ethereum.geth),
            tokens: Some(ethereum.tokens.into()),
        }
    }
}

impl From<Tokens> for file::Tokens {
    fn from(tokens: Tokens) -> Self {
        file::Tokens {
            dai: Some(tokens.dai),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Geth {
    pub node_url: Url,
}

impl Geth {
    fn new() -> Self {
        Self {
            node_url: WEB3_URL.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Tokens {
    pub dai: ethereum::Address,
}

impl Tokens {
    fn new(chain_id: ChainId) -> Result<Self> {
        let dai = dai_address_from_chain_id(chain_id)?;

        Ok(Self { dai })
    }

    fn from_file(file: file::Tokens, id: ChainId) -> Result<Self> {
        let dai = file.dai.map_or_else(|| dai_address_from_chain_id(id), Ok)?;

        Ok(Self { dai })
    }
}

fn dai_address_from_chain_id(id: ChainId) -> Result<ethereum::Address> {
    Ok(match id {
        ChainId::MAINNET => *DAI_MAINNET,
        ChainId::ROPSTEN => *DAI_ROPSTEN,
        ChainId::KOVAN => *DAI_KOVAN,
        id => anyhow::bail!(
            "unable to infer DAI token contract from chain-ID {}",
            u32::from(id)
        ),
    })
}

fn parse_unchecked<T>(str: &'static str) -> T
where
    T: FromStr + Debug,
    <T as FromStr>::Err: Send + Sync + 'static + std::error::Error,
{
    str.parse()
        .with_context(|| format!("failed to parse static string '{}' into T", str))
        .unwrap()
}
