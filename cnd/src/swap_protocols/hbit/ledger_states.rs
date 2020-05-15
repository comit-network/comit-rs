use crate::swap_protocols::{
    hbit::{LedgerState, SwapEvent},
    LocalSwapId,
};
use std::collections::HashMap;
use tokio::sync::Mutex;

#[derive(Default, Debug)]
pub struct LedgerStates {
    states: Mutex<HashMap<LocalSwapId, LedgerState>>,
}

impl LedgerStates {
    pub async fn insert(&self, key: LocalSwapId, value: LedgerState) {
        let mut states = self.states.lock().await;
        states.insert(key, value);
    }

    pub async fn get(&self, key: &LocalSwapId) -> Option<LedgerState> {
        let states = self.states.lock().await;
        states.get(key).cloned()
    }

    pub async fn update(&self, key: &LocalSwapId, event: SwapEvent) {
        let mut states = self.states.lock().await;
        let ledger_state = match states.get_mut(key) {
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
    use spectral::prelude::*;

    #[tokio::test]
    async fn insert_and_get_ledger_state() {
        let ledger_states = LedgerStates::default();
        let id = LocalSwapId::default();

        ledger_states.insert(id, LedgerState::NotDeployed).await;

        let res: Option<LedgerState> = ledger_states.get(&id).await;
        assert_that(&res).contains_value(&LedgerState::NotDeployed);
    }
}
