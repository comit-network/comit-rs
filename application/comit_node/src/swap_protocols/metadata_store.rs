use std::{collections::HashMap, fmt::Display, hash::Hash, sync::Mutex};

#[derive(Clone, Copy, Debug, Display)]
pub enum RoleKind {
    Alice,
    Bob,
}

#[derive(Clone, Copy, Debug, Display)]
pub enum LedgerKind {
    Bitcoin,
    Ethereum,
}

#[derive(Clone, Copy, Debug, Display)]
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

impl<K: Hash + Eq + Clone + Send + Sync + 'static> MetadataStore<K> for InMemoryMetadataStore<K> {
    fn get(&self, key: &K) -> Result<Option<Metadata>, Error> {
        let metadata = self.metadata.lock().unwrap();
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
            .map(|(key, value)| (key.clone(), *value))
            .collect())
    }
}
