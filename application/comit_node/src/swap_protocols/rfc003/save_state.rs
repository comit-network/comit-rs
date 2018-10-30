use futures::sync::mpsc;
use std::sync::RwLock;
use swap_protocols::rfc003::{state_machine::SwapStates, Ledger, SecretHash};

pub trait SaveState<SL: Ledger, TL: Ledger, SA: Clone, TA: Clone, S: Into<SecretHash> + Clone>:
    Send + Sync
{
    fn save(&self, state: SwapStates<SL, TL, SA, TA, S>);
}

impl<
        SL: Ledger,
        TL: Ledger,
        SA: Clone + Send + Sync,
        TA: Clone + Send + Sync,
        S: Into<SecretHash> + Clone + Send + Sync,
    > SaveState<SL, TL, SA, TA, S> for RwLock<SwapStates<SL, TL, SA, TA, S>>
{
    fn save(&self, state: SwapStates<SL, TL, SA, TA, S>) {
        let _self = &mut *self.write().unwrap();
        *_self = state;
    }
}

impl<
        SL: Ledger,
        TL: Ledger,
        SA: Clone + Send + Sync,
        TA: Clone + Send + Sync,
        S: Into<SecretHash> + Clone + Send + Sync,
    > SaveState<SL, TL, SA, TA, S> for mpsc::UnboundedSender<SwapStates<SL, TL, SA, TA, S>>
{
    fn save(&self, state: SwapStates<SL, TL, SA, TA, S>) {
        // ignore error the subscriber is no longer interested in state updates
        let _ = self.unbounded_send(state);
    }
}
