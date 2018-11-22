use comit_client::SwapReject;
use futures::{
    sync::oneshot::{self, Sender},
    Future,
};
use std::{any::Any, collections::HashMap, hash::Hash, sync::Mutex};
use swap_protocols::{
    asset::Asset,
    rfc003::{events::ResponseFuture, roles::Bob, state_machine::StateMachineResponse, Ledger},
};

#[allow(type_alias_bounds)]
type PendingResponseSender<SL: Ledger, TL: Ledger> =
    Sender<Result<StateMachineResponse<SL::Identity, TL::Identity, TL::LockDuration>, SwapReject>>;

#[derive(Debug, Default)]
pub struct PendingResponses<K: Hash + Eq> {
    pending_responses: Mutex<HashMap<K, Box<Any + Send + 'static>>>,
}

impl<K: Hash + Eq + Clone + Send + Sync + 'static> PendingResponses<K> {
    pub fn create<SL: Ledger, TL: Ledger, SA: Asset, TA: Asset>(
        &self,
        key: K,
    ) -> Box<ResponseFuture<Bob<SL, TL, SA, TA>>> {
        let (sender, receiver) = oneshot::channel();

        let mut pending_responses = match self.pending_responses.lock() {
            Ok(guard) => guard,
            Err(e) => e.into_inner(),
        };

        let _old = pending_responses.insert(key, Box::new(sender));

        Box::new(receiver.map_err(|_| unreachable!("The sender should never be dropped")))
    }

    pub fn take<SL: Ledger, TL: Ledger>(&self, key: &K) -> Option<PendingResponseSender<SL, TL>> {
        let mut pending_responses = match self.pending_responses.lock() {
            Ok(guard) => guard,
            Err(e) => e.into_inner(),
        };

        pending_responses.remove(key).and_then(|sender| {
            match sender.downcast::<PendingResponseSender<SL, TL>>() {
                Ok(sender) => Some(*sender),
                Err(e) => {
                    error!("Failed to downcast sender to expected type: {:?}", e);
                    None
                }
            }
        })
    }
}
