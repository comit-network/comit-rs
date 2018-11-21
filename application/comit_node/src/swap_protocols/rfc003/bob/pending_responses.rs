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

        let mut pending_responses = self.pending_responses.lock().unwrap();

        let _old = pending_responses.insert(key, Box::new(sender));

        Box::new(receiver.map_err(|_| unreachable!("The sender should never be dropped")))
    }

    pub fn take<SL: Ledger, TL: Ledger>(
        &self,
        key: &K,
    ) -> Option<
        Sender<
            Result<StateMachineResponse<SL::Identity, TL::Identity, TL::LockDuration>, SwapReject>,
        >,
    > {
        let mut pending_responses = self.pending_responses.lock().unwrap();
        pending_responses.remove(key).map(|sender| {
            let sender = sender
                .downcast::<Sender<
                    Result<
                        StateMachineResponse<SL::Identity, TL::Identity, TL::LockDuration>,
                        SwapReject,
                    >,
                >>()
                .unwrap();
            *sender
        })
    }
}
