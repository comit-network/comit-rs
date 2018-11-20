use std::{
    any::Any,
    collections::HashMap,
    hash::Hash,
    sync::{Arc, Mutex, RwLock},
};
use swap_protocols::rfc003::{roles::Role, state_machine::SwapStates, SaveState};

#[derive(Debug)]
pub enum Error {
    NotFound,
    DuplicateKey,
}

pub trait StateStore<K>: Send + Sync + 'static {
    fn insert<R: Role>(&self, key: K, state: SwapStates<R>) -> Result<Arc<SaveState<R>>, Error>;

    fn get<R: Role>(&self, key: &K) -> Result<SwapStates<R>, Error>;

    #[allow(clippy::type_complexity)]
    fn save_state_for_key<R: Role>(&self, key: &K) -> Result<Arc<SaveState<R>>, Error>;
}

#[derive(Default, Debug)]
pub struct InMemoryStateStore<K: Hash + Eq> {
    states: Mutex<HashMap<K, Box<Any + Send + Sync>>>,
}

impl<K: Hash + Eq + Clone + Send + Sync + 'static> StateStore<K> for InMemoryStateStore<K> {
    fn insert<R: Role>(&self, key: K, state: SwapStates<R>) -> Result<Arc<SaveState<R>>, Error> {
        let mut states = self.states.lock().unwrap();

        if states.contains_key(&key) {
            return Err(Error::DuplicateKey);
        }

        let state = Arc::new(RwLock::new(state));

        let value: Box<Any + Send + Sync> = Box::new(state.clone());
        let _old = states.insert(key, value);

        Ok(state)
    }

    fn get<R: Role>(&self, key: &K) -> Result<SwapStates<R>, Error> {
        let states = self.states.lock().unwrap();
        states
            .get(key)
            .map(|state| {
                let state = state.downcast_ref::<Arc<RwLock<SwapStates<R>>>>().unwrap();
                let state = state.read().unwrap();
                state.clone()
            })
            .ok_or(Error::NotFound)
    }

    fn save_state_for_key<R: Role>(&self, key: &K) -> Result<Arc<SaveState<R>>, Error> {
        let states = self.states.lock().unwrap();
        states
            .get(key)
            .map(|state| -> Arc<SaveState<R>> {
                let state = state.downcast_ref::<Arc<RwLock<SwapStates<R>>>>().unwrap();
                state.clone()
            })
            .ok_or(Error::NotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin_support::{BitcoinQuantity, Blocks};
    use ethereum_support::EtherQuantity;
    use hex;
    use secp256k1_support;
    use spectral::prelude::*;
    use swap_protocols::{
        ledger::{Bitcoin, Ethereum},
        rfc003::{roles::test::Alisha, state_machine::Start, Secret},
    };

    #[test]
    fn store_get_and_save_state() {
        let state_store = InMemoryStateStore::default();
        let start_state = Start::<Alisha> {
            alpha_ledger_refund_identity: secp256k1_support::KeyPair::from_secret_key_slice(
                &hex::decode("18e14a7b6a307f426a94f8114701e7c8e774e7f9a47e2c2035db29a206321725")
                    .unwrap(),
            )
            .unwrap(),
            beta_ledger_success_identity: "8457037fcd80a8650c4692d7fcfc1d0a96b92867"
                .parse()
                .unwrap(),
            alpha_ledger: Bitcoin::regtest(),
            beta_ledger: Ethereum::default(),
            alpha_asset: BitcoinQuantity::from_bitcoin(1.0),
            beta_asset: EtherQuantity::from_eth(10.0),
            alpha_ledger_lock_duration: Blocks::from(144),
            secret: Secret::from(*b"hello world, you are beautiful!!"),
        };

        let state = SwapStates::from(start_state.clone());
        let id = 1;

        let res = state_store.insert(id, state.clone());
        assert!(res.is_ok());

        let res = state_store.get(&id);
        assert_that(&res).is_ok_containing(state);

        let save_state = state_store.save_state_for_key(&id).unwrap();

        let second_state = SwapStates::from(Start {
            secret: Secret::from(*b"!!lufituaeb era uoy ,dlrow olleh"),
            ..start_state
        });

        save_state.save(second_state.clone());

        let res = state_store.get(&id);
        assert_that(&res).is_ok().is_equal_to(second_state)
    }
}
