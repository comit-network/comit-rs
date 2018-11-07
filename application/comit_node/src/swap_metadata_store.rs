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
pub struct Types {
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
    fn get(&self, key: &K) -> Result<Types, Error>;
    fn add(&self, key: K, types: Types) -> Result<(), Error>;
}

#[derive(Debug, Default)]
pub struct InMemorySwapMetadataStore<K: Hash + Eq> {
    types: Mutex<HashMap<K, Types>>,
}

impl<K: Hash + Eq + Clone + Send + Sync + 'static> SwapMetadataStore<K>
    for InMemorySwapMetadataStore<K>
{
    fn get(&self, key: &K) -> Result<Types, Error> {
        let types = self.types.lock().unwrap();
        match types.get(&key) {
            Some(types) => Ok(*types),
            None => Err(Error::NotFound),
        }
    }

    fn add(&self, key: K, value: Types) -> Result<(), Error> {
        let mut types = self.types.lock().unwrap();

        let old_types = types.insert(key, value);
        match old_types {
            Some(_) => Err(Error::DuplicateKey),
            None => Ok(()),
        }
    }
}
