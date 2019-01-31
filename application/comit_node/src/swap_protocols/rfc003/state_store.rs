use crate::swap_protocols::rfc003::{state_machine::SwapStates, Role, SaveState};
use std::{
    any::Any,
    collections::HashMap,
    hash::Hash,
    sync::{Arc, Mutex, RwLock},
};

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "State already exists for given key")]
    DuplicateKey,
}

pub trait StateStore<K>: Send + Sync + 'static {
    fn new_save_state<R: Role>(&self, key: K) -> Result<Arc<dyn SaveState<R>>, Error>;

    fn get<R: Role>(&self, key: &K) -> Result<Option<SwapStates<R>>, Error>;

    #[allow(clippy::type_complexity)]
    fn get_save_state<R: Role>(&self, key: &K) -> Result<Option<Arc<dyn SaveState<R>>>, Error>;
}

#[derive(Default, Debug)]
pub struct InMemoryStateStore<K: Hash + Eq> {
    states: Mutex<HashMap<K, Box<dyn Any + Send + Sync>>>,
}

impl<K: Hash + Eq + Clone + Send + Sync + 'static> StateStore<K> for InMemoryStateStore<K> {
    fn new_save_state<R: Role>(&self, key: K) -> Result<Arc<dyn SaveState<R>>, Error> {
        let mut states = self.states.lock().unwrap();

        if states.contains_key(&key) {
            return Err(Error::DuplicateKey);
        }

        let state = Arc::new(RwLock::new(None));
        let value: Box<dyn Any + Send + Sync> = Box::new(Arc::clone(&state));
        let _ = states.insert(key, value);

        Ok(state)
    }

    fn get<R: Role>(&self, key: &K) -> Result<Option<SwapStates<R>>, Error> {
        let states = self.states.lock().unwrap();
        Ok(states.get(key).and_then(|state| {
            match state.downcast_ref::<Arc<RwLock<Option<SwapStates<R>>>>>() {
                Some(state) => state.read().unwrap().clone(),
                None => {
                    error!("Attempted to get state with wrong type");
                    None
                }
            }
        }))
    }

    fn get_save_state<R: Role>(&self, key: &K) -> Result<Option<Arc<dyn SaveState<R>>>, Error> {
        let states = self.states.lock().unwrap();
        match states.get(key) {
            Some(state) => Ok(state
                .downcast_ref::<Arc<RwLock<Option<SwapStates<R>>>>>()
                .map(|save_state| save_state.clone() as Arc<dyn SaveState<R>>)),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::swap_protocols::{
        ledger::{Bitcoin, Ethereum},
        rfc003::{role::test::Alisha, state_machine::Start, Secret, Timestamp},
    };
    use bitcoin_support::BitcoinQuantity;
    use ethereum_support::EtherQuantity;
    use spectral::prelude::*;

    #[test]
    fn store_get_and_save_state() {
        let state_store = InMemoryStateStore::default();
        let start_state = Start::<Alisha> {
            alpha_ledger_refund_identity: secp256k1_support::KeyPair::from_secret_key_slice(
                &hex::decode("18e14a7b6a307f426a94f8114701e7c8e774e7f9a47e2c2035db29a206321725")
                    .unwrap(),
            )
            .unwrap(),
            beta_ledger_redeem_identity: "8457037fcd80a8650c4692d7fcfc1d0a96b92867"
                .parse()
                .unwrap(),
            alpha_ledger: Bitcoin::default(),
            beta_ledger: Ethereum::default(),
            alpha_asset: BitcoinQuantity::from_bitcoin(1.0),
            beta_asset: EtherQuantity::from_eth(10.0),
            alpha_expiry: Timestamp::from(2000000000),
            beta_expiry: Timestamp::from(2000000000),
            secret: Secret::from(*b"hello world, you are beautiful!!"),
            role: Alisha::default(),
        };

        let state = SwapStates::from(start_state.clone());
        let id = 1;

        let res = state_store.new_save_state(id);
        assert!(res.is_ok());

        let save_state = res.unwrap();
        save_state.save(start_state.clone().into());

        let res = state_store.get(&id).unwrap();
        assert_that(&res).contains_value(state);

        let save_state = state_store.get_save_state(&id).unwrap().unwrap();

        let second_state = SwapStates::from(Start {
            secret: Secret::from(*b"!!lufituaeb era uoy ,dlrow olleh"),
            ..start_state
        });

        save_state.save(second_state.clone());

        let res = state_store.get(&id).unwrap();
        assert_that(&res).contains_value(second_state)
    }
}
