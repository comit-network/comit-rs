use std::{collections::HashMap, hash::Hash, sync::Mutex};

#[derive(Clone, Copy, Debug)]
pub enum Roles {
    Alice,
    Bob,
}

#[derive(Clone, Copy, Debug)]
pub enum Ledgers {
    Bitcoin,
    Ethereum,
}

#[derive(Clone, Copy, Debug)]
pub enum Assets {
    Bitcoin,
    Ether,
}

#[derive(Clone, Copy, Debug)]
pub struct Metadata {
    pub source_ledger: Ledgers,
    pub target_ledger: Ledgers,
    pub source_asset: Assets,
    pub target_asset: Assets,
    pub role: Roles,
}

#[derive(Debug)]
pub enum Error {
    NotFound,
    DuplicateKey,
}

pub trait MetadataStore<K>: Send + Sync + 'static {
    fn get(&self, key: &K) -> Result<Metadata, Error>;
    fn insert(&self, key: K, metadata: Metadata) -> Result<(), Error>;
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

    fn insert(&self, key: K, value: Metadata) -> Result<(), Error> {
        let mut metadata = self.metadata.lock().unwrap();

        if metadata.contains_key(&key) {
            return Err(Error::DuplicateKey);
        }

        let _ = metadata.insert(key, value);
        Ok(())
    }
}
