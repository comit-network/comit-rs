use crate::{
    db,
    swap_protocols::{asset, ledger, swap_id::SwapId, Role},
};
use libp2p::{self, PeerId};
use std::{collections::HashMap, sync::Mutex};
use strum_macros::{Display, EnumString};

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
    DuplicateKey,
    Sqlite(db::Error),
}

impl From<db::Error> for Error {
    fn from(err: db::Error) -> Error {
        Error::Sqlite(err)
    }
}

pub trait MetadataStore: Send + Sync + 'static {
    fn get(&self, key: SwapId) -> Result<Option<Metadata>, Error>;
    fn insert(&self, metadata: Metadata) -> Result<(), Error>;
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

    fn insert(&self, value: Metadata) -> Result<(), Error> {
        let mut metadata = self.metadata.lock().unwrap();
        let key = value.swap_id;

        if metadata.contains_key(&key) {
            return Err(Error::DuplicateKey);
        }

        let _ = metadata.insert(key, value);
        Ok(())
    }
    fn all(&self) -> Result<Vec<Metadata>, Error> {
        let metadata = self.metadata.lock().unwrap();

        Ok(metadata.iter().map(|(_key, value)| value.clone()).collect())
    }
}
