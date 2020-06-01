use crate::{
    swap_protocols::{
        rfc003::{create_swap::SwapEvent, LedgerState},
        state::{Get, Insert, Update},
    },
    LocalSwapId,
};
use async_trait::async_trait;
use std::{any::Any, collections::HashMap};
use tokio::sync::Mutex;

#[derive(Default, Debug)]
pub struct LedgerStates {
    states: Mutex<HashMap<LocalSwapId, Box<dyn Any + Send>>>,
}

#[async_trait]
impl<S> Insert<S> for LedgerStates
where
    S: Send + 'static,
{
    async fn insert(&self, key: LocalSwapId, value: S) {
        let mut states = self.states.lock().await;
        states.insert(key, Box::new(value));
    }
}

#[async_trait]
impl<S> Get<S> for LedgerStates
where
    S: Clone + Send + 'static,
{
    async fn get(&self, key: &LocalSwapId) -> anyhow::Result<Option<S>> {
        let states = self.states.lock().await;
        match states.get(key) {
            Some(state) => match state.downcast_ref::<S>() {
                Some(state) => Ok(Some(state.clone())),
                None => Err(anyhow::anyhow!("invalid type")),
            },
            None => Ok(None),
        }
    }
}

#[async_trait]
impl<A, H, T> Update<SwapEvent<A, H, T>> for LedgerStates
where
    LedgerState<A, H, T>: 'static,
    A: Send,
    H: Send,
    T: Send,
{
    async fn update(&self, key: &LocalSwapId, event: SwapEvent<A, H, T>) {
        let mut states = self.states.lock().await;
        let ledger_state = match states
            .get_mut(key)
            .and_then(|state| state.downcast_mut::<LedgerState<A, H, T>>())
        {
            Some(state) => state,
            None => {
                tracing::warn!("Value not found for key {}", key);
                return;
            }
        };

        match event {
            SwapEvent::Deployed(deployed) => ledger_state.transition_to_deployed(deployed),
            SwapEvent::Funded(funded) => ledger_state.transition_to_funded(funded),
            SwapEvent::Redeemed(redeemed) => {
                // what if redeemed.secret.hash() != secret_hash in request ??

                ledger_state.transition_to_redeemed(redeemed);
            }
            SwapEvent::Refunded(refunded) => ledger_state.transition_to_refunded(refunded),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{asset, htlc_location, transaction};
    use spectral::prelude::*;

    #[tokio::test]
    async fn insert_and_get_ledger_state() {
        let ledger_states = LedgerStates::default();
        let id = LocalSwapId::default();

        ledger_states.insert(id, LedgerState::<asset::Bitcoin, htlc_location::Bitcoin, transaction::Bitcoin>::NotDeployed).await;

        let res: Option<LedgerState<asset::Bitcoin, htlc_location::Bitcoin, transaction::Bitcoin>> =
            ledger_states.get(&id).await.unwrap();
        assert_that(&res).contains_value(&LedgerState::NotDeployed);
    }
}
