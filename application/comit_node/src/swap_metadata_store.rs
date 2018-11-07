use std::{collections::HashMap, hash::Hash, sync::Mutex};

#[derive(Clone, Copy, Debug)]
pub enum Role {
    Alice,
    Bob,
}

#[derive(Clone, Copy, Debug)]
pub enum Ledger {
    Bitcoin,
    Ethereum,
}

#[derive(Clone, Copy, Debug)]
pub enum Asset {
    Bitcoin,
    Ether,
}

#[derive(Clone, Copy, Debug)]
pub struct SwapMetadata {
    pub source_ledger: Ledger,
    pub target_ledger: Ledger,
    pub source_asset: Asset,
    pub target_asset: Asset,
    pub role: Role,
}

#[derive(Debug)]
pub enum Error {
    NotFound,
    DuplicateKey,
}

pub trait SwapMetadataStore<K>: Send + Sync + 'static {
    fn get(&self, key: &K) -> Result<SwapMetadata, Error>;
    fn insert(&self, key: K, swap_metadata: SwapMetadata) -> Result<(), Error>;
}

#[derive(Debug, Default)]
pub struct InMemorySwapMetadataStore<K: Hash + Eq> {
    swap_metadata: Mutex<HashMap<K, SwapMetadata>>,
}

impl<K: Hash + Eq + Clone + Send + Sync + 'static> SwapMetadataStore<K>
    for InMemorySwapMetadataStore<K>
{
    fn get(&self, key: &K) -> Result<SwapMetadata, Error> {
        let swap_metadata = self.swap_metadata.lock().unwrap();
        match swap_metadata.get(&key) {
            Some(swap_metadata) => Ok(*swap_metadata),
            None => Err(Error::NotFound),
        }
    }

    fn insert(&self, key: K, value: SwapMetadata) -> Result<(), Error> {
        let mut swap_metadata = self.swap_metadata.lock().unwrap();

        if swap_metadata.contains_key(&key) {
            return Err(Error::DuplicateKey);
        }

        let _ = swap_metadata.insert(key, value);
        Ok(())
    }
}
