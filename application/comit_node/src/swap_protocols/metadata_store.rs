use std::{collections::HashMap, hash::Hash, sync::Mutex};

#[derive(Clone, Copy, Debug)]
pub enum RoleKind {
    Alice,
    Bob,
}

#[derive(Clone, Copy, Debug)]
pub enum LedgerKind {
    Bitcoin,
    Ethereum,
}

#[derive(Clone, Copy, Debug)]
pub enum AssetKind {
    Bitcoin,
    Ether,
    Erc20,
}

#[derive(Clone, Copy, Debug)]
pub struct Metadata {
    pub alpha_ledger: LedgerKind,
    pub beta_ledger: LedgerKind,
    pub alpha_asset: AssetKind,
    pub beta_asset: AssetKind,
    pub role: RoleKind,
}

#[derive(Debug)]
pub enum Error {
    NotFound,
    DuplicateKey,
}

pub trait MetadataStore<K>: Send + Sync + 'static {
    fn get(&self, key: &K) -> Result<Metadata, Error>;
    fn insert<M: Into<Metadata>>(&self, key: K, metadata: M) -> Result<(), Error>;
}

#[derive(Debug, Default)]
pub struct InMemoryMetadataStore<K: Hash + Eq> {
    metadata: Mutex<HashMap<K, Metadata>>,
}

impl<K: Hash + Eq + Clone + Send + Sync + 'static> MetadataStore<K> for InMemoryMetadataStore<K> {
    fn get(&self, key: &K) -> Result<Metadata, Error> {
        let metadata = self.metadata.lock().unwrap();
        match metadata.get(&key) {
            Some(metadata) => Ok(*metadata),
            None => Err(Error::NotFound),
        }
    }

    fn insert<M: Into<Metadata>>(&self, key: K, value: M) -> Result<(), Error> {
        let mut metadata = self.metadata.lock().unwrap();

        if metadata.contains_key(&key) {
            return Err(Error::DuplicateKey);
        }

        let _ = metadata.insert(key, value.into());
        Ok(())
    }
}
