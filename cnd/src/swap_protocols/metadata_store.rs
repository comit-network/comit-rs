use crate::swap_protocols::{asset::AssetKind, swap_id::SwapId, LedgerKind};
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

#[derive(Clone, Debug)]
pub struct Metadata {
    pub alpha_ledger: LedgerKind,
    pub beta_ledger: LedgerKind,
    pub alpha_asset: AssetKind,
    pub beta_asset: AssetKind,
    pub role: RoleKind,
    pub counterparty: PeerId,
    pub swap_id: SwapId,
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
