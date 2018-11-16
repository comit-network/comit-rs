use futures::sync::mpsc;
use std::sync::RwLock;
use swap_protocols::rfc003::{roles::Role, state_machine::SwapStates};

pub trait SaveState<R: Role>: Send + Sync {
    fn save(&self, state: SwapStates<R>);
}

impl<R: Role> SaveState<R> for RwLock<SwapStates<R>> {
    fn save(&self, state: SwapStates<R>) {
        let _self = &mut *self.write().unwrap();
        *_self = state;
    }
}

impl<R: Role> SaveState<R> for mpsc::UnboundedSender<SwapStates<R>> {
    fn save(&self, state: SwapStates<R>) {
        // ignore error the subscriber is no longer interested in state updates
        let _ = self.unbounded_send(state);
    }
}
