use crate::swap_protocols::{asset, ledger, swap_id::SwapId};
use libp2p::{self, PeerId};
use std::{collections::HashMap, fmt, sync::Mutex};
use strum;
use strum_macros::{Display, EnumString};
use uuid::parser;

#[derive(Clone, Copy, Debug, Display, EnumString)]
pub enum Role {
    Alice,
    Bob,
}

#[derive(Debug, Clone, Display, EnumString)]
pub enum LedgerKind {
    Bitcoin,
    Ethereum,
}

impl From<ledger::LedgerKind> for LedgerKind {
    fn from(ledger: ledger::LedgerKind) -> LedgerKind {
        match ledger {
            ledger::LedgerKind::Bitcoin(_) => LedgerKind::Bitcoin,
            ledger::LedgerKind::Ethereum(_) => LedgerKind::Ethereum,
            // In order to remove this ledger::LedgerKind::Unknown should be removed.
            // Doing so requires handling unknown ledger during deserialization.
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Debug, Display, EnumString)]
pub enum AssetKind {
    Bitcoin,
    Ether,
    Erc20,
}

impl From<asset::AssetKind> for AssetKind {
    fn from(asset: asset::AssetKind) -> AssetKind {
        match asset {
            asset::AssetKind::Bitcoin(_) => AssetKind::Bitcoin,
            asset::AssetKind::Ether(_) => AssetKind::Ether,
            asset::AssetKind::Erc20(_) => AssetKind::Erc20,
            // In order to remove this ledger::AssetKind::Unknown should be removed.
            // Doing so requires handling unknown asset during deserialization.
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Metadata {
    pub swap_id: SwapId,
    pub alpha_ledger: LedgerKind,
    pub beta_ledger: LedgerKind,
    pub alpha_asset: AssetKind,
    pub beta_asset: AssetKind,
    pub role: Role,
    pub counterparty: PeerId,
}

impl Metadata {
    pub fn new(
        swap_id: SwapId,
        al: ledger::LedgerKind,
        bl: ledger::LedgerKind,
        aa: asset::AssetKind,
        ba: asset::AssetKind,
        role: Role,
        counterparty: PeerId,
    ) -> Metadata {
        Metadata {
            swap_id,
            alpha_ledger: al.into(),
            beta_ledger: bl.into(),
            alpha_asset: aa.into(),
            beta_asset: ba.into(),
            role,
            counterparty,
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Path(String),
    Init(String),
    Connect(String),
    Load(String),
    Insert(String),
    Parse(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // FIXME: is this idiomatic [CoBloX] Rust?
            Error::Path(msg) => write!(f, "Datastore path error: {}", msg),
            Error::Init(msg) => write!(f, "Failed to initialize datastore : {}", msg),
            Error::Connect(msg) => write!(f, "Failed to connect to datastore: {}", msg),
            Error::Load(msg) => write!(f, "Failed to load record: {}", msg),
            Error::Insert(msg) => write!(f, "Failed to insert new record: {}", msg),
            Error::Parse(msg) => write!(f, "Failed to parse stored record: {}", msg),
        }
    }
}

// FIXME: Do we need this?
impl std::error::Error for Error {
    fn description(&self) -> &str {
        "MetadataStore error"
    }

    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None // FIXME: What goes here?
    }
}

impl From<strum::ParseError> for Error {
    fn from(err: strum::ParseError) -> Error {
        Error::Parse(err.to_string())
    }
}

impl From<parser::ParseError> for Error {
    fn from(err: parser::ParseError) -> Error {
        Error::Parse(err.to_string())
    }
}

pub trait MetadataStore: Send + Sync + 'static {
    fn get(&self, key: SwapId) -> Result<Option<Metadata>, Error>;
    fn insert<M: Into<Metadata>>(&self, metadata: M) -> Result<(), Error>;
    fn all(&self) -> Result<Vec<Metadata>, Error>;
}

#[derive(Debug, Default)]
pub struct InMemoryMetadataStore {
    metadata: Mutex<HashMap<SwapId, Metadata>>,
}

impl MetadataStore for InMemoryMetadataStore {
    fn get(&self, key: SwapId) -> Result<Option<Metadata>, Error> {
        let metadata = self.metadata.lock().unwrap();
        log::trace!("Fetched metadata of swap with id {:?}: {:?}", key, metadata);

        Ok(metadata.get(&key).map(Clone::clone))
    }

    fn insert<M: Into<Metadata>>(&self, value: M) -> Result<(), Error> {
        let mut metadata = self.metadata.lock().unwrap();
        let value: Metadata = value.into();
        let key = value.swap_id;

        if metadata.contains_key(&key) {
            return Err(Error::Insert("key (swap id) already exists".to_string()));
        }

        let _ = metadata.insert(key, value.into());
        Ok(())
    }
    fn all(&self) -> Result<Vec<Metadata>, Error> {
        let metadata = self.metadata.lock().unwrap();

        Ok(metadata.iter().map(|(_key, value)| value.clone()).collect())
    }
}
