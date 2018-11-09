use comit_client::SwapReject;
use futures::{
    sync::oneshot::{self, Sender},
    Future,
};
use std::{any::Any, collections::HashMap, hash::Hash, sync::Mutex};
use swap_protocols::{
    asset::Asset,
    rfc003::{events::ResponseFuture, messages::AcceptResponseBody, roles::Bob, Ledger},
};

pub type SenderAction<SL, TL> = Sender<Result<AcceptResponseBody<SL, TL>, SwapReject>>;

#[derive(Debug, Default)]
pub struct PendingResponseStore<K: Hash + Eq> {
    pending_responses: Mutex<HashMap<K, Box<Any + Send + 'static>>>,
}

impl<K: Hash + Eq + Clone + Send + Sync + 'static> PendingResponseStore<K> {
    pub fn create<SL: Ledger, TL: Ledger, SA: Asset, TA: Asset>(
        &self,
        key: K,
    ) -> Box<ResponseFuture<Bob<SL, TL, SA, TA>>> {
        let (sender, receiver) = oneshot::channel();

        let mut pending_responses = self.pending_responses.lock().unwrap();

        let _old = pending_responses.insert(key, Box::new(sender));

        Box::new(receiver.map_err(|_| unreachable!("The sender should never be dropped")))
    }

    pub fn take<SL: Ledger, TL: Ledger>(&self, key: &K) -> Option<SenderAction<SL, TL>> {
        let mut pending_responses = self.pending_responses.lock().unwrap();
        pending_responses.remove(key).map(|sender| {
            let sender = sender.downcast::<SenderAction<SL, TL>>().unwrap();
            *sender
        })
    }
}
