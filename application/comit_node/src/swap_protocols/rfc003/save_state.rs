use futures::sync::mpsc;
use std::{fmt::Debug, sync::RwLock};
use swap_protocols::{
    asset::Asset,
    rfc003::{state_machine::SwapStates, IntoSecretHash, Ledger},
};

pub trait SaveState<SL: Ledger, TL: Ledger, SA: Asset, TA: Asset, S: IntoSecretHash>:
    Send + Sync + Debug
{
    fn save(&self, state: SwapStates<SL, TL, SA, TA, S>);
}

impl<SL: Ledger, TL: Ledger, SA: Asset, TA: Asset, S: IntoSecretHash> SaveState<SL, TL, SA, TA, S>
    for RwLock<SwapStates<SL, TL, SA, TA, S>>
{
    fn save(&self, state: SwapStates<SL, TL, SA, TA, S>) {
        let _self = &mut *self.write().unwrap();
        *_self = state;
    }
}

impl<SL: Ledger, TL: Ledger, SA: Asset, TA: Asset, S: IntoSecretHash> SaveState<SL, TL, SA, TA, S>
    for mpsc::UnboundedSender<SwapStates<SL, TL, SA, TA, S>>
{
    fn save(&self, state: SwapStates<SL, TL, SA, TA, S>) {
        // ignore error the subscriber is no longer interested in state updates
        let _ = self.unbounded_send(state);
    }
}
