use futures::sync::mpsc;
use std::sync::RwLock;
use swap_protocols::rfc003::{alice, Ledger};

pub trait SaveState<S>: Send + Sync {
    fn save(&self, state: S);
}

impl<SL: Ledger, TL: Ledger, SA: Clone + Send + Sync, TA: Clone + Send + Sync>
    SaveState<alice::SwapStates<SL, TL, SA, TA>> for RwLock<alice::SwapStates<SL, TL, SA, TA>>
{
    fn save(&self, state: alice::SwapStates<SL, TL, SA, TA>) {
        let _self = &mut *self.write().unwrap();
        *_self = state;
    }
}

impl<SL: Ledger, TL: Ledger, SA: Clone + Send + Sync, TA: Clone + Send + Sync>
    SaveState<alice::SwapStates<SL, TL, SA, TA>>
    for mpsc::UnboundedSender<alice::SwapStates<SL, TL, SA, TA>>
{
    fn save(&self, state: alice::SwapStates<SL, TL, SA, TA>) {
        // ignore error the subscriber is no longer interested in state updates
        let _ = self.unbounded_send(state);
    }
}
