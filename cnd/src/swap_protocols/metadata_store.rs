use crate::swap_protocols::{asset, ledger, swap_id::SwapId};
use failure::Fail;
use libp2p::PeerId;
use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    hash::Hash,
    sync::Mutex,
};

#[derive(Clone, Copy, Debug, strum_macros::Display)]
pub enum RoleKind {
    Alice,
    Bob,
}

#[derive(Debug, Clone)]
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

#[derive(Clone, Debug)]
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
    pub role: RoleKind,
    pub counterparty: PeerId,
}

impl Metadata {
    pub fn new(
        swap_id: SwapId,
        al: ledger::LedgerKind,
        bl: ledger::LedgerKind,
        aa: asset::AssetKind,
        ba: asset::AssetKind,
        role: RoleKind,
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

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "Metadata already exists")]
    DuplicateKey,
}

pub trait MetadataStore<K>: Send + Sync + 'static {
    fn get(&self, key: &K) -> Result<Option<Metadata>, Error>;
    fn insert<M: Into<Metadata>>(&self, key: K, metadata: M) -> Result<(), Error>;
    fn all(&self) -> Result<Vec<(K, Metadata)>, Error>;
}

#[derive(Debug, Default)]
pub struct InMemoryMetadataStore<K: Hash + Eq> {
    metadata: Mutex<HashMap<K, Metadata>>,
}

impl<K: Debug + Display + Hash + Eq + Clone + Send + Sync + 'static> MetadataStore<K>
    for InMemoryMetadataStore<K>
{
    fn get(&self, key: &K) -> Result<Option<Metadata>, Error> {
        let metadata = self.metadata.lock().unwrap();
        log::trace!("Fetched metadata of swap with id {}: {:?}", key, metadata);

        Ok(metadata.get(&key).map(Clone::clone))
    }

    fn insert<M: Into<Metadata>>(&self, key: K, value: M) -> Result<(), Error> {
        let mut metadata = self.metadata.lock().unwrap();

        if metadata.contains_key(&key) {
            return Err(Error::DuplicateKey);
        }

        let _ = metadata.insert(key, value.into());
        Ok(())
    }
    fn all(&self) -> Result<Vec<(K, Metadata)>, Error> {
        let metadata = self.metadata.lock().unwrap();

        Ok(metadata
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect())
    }
}
