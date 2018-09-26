use comit_client::{Client, SwapReject};
use common_types::seconds::Seconds;
use futures::{
    future,
    sync::oneshot::{self, Sender},
    Future,
};
use std::{
    any::{Any, TypeId},
    borrow::Borrow,
    collections::HashMap,
    str::FromStr,
    sync::Mutex,
};
use swap_protocols::{ledger::Ledger, rfc003, wire_types};
use transport_protocol::{self, json};

#[allow(dead_code)]
pub struct FakeClient {
    pending_requests: Mutex<HashMap<TypeId, Sender<Box<Any + Send>>>>,
}

impl FakeClient {
    pub fn new() -> Self {
        FakeClient {
            pending_requests: Mutex::new(HashMap::new()),
        }
    }

    pub fn resolve_request<SL: Ledger, TL: Ledger>(
        &self,
        response: Result<rfc003::AcceptResponse<SL, TL>, SwapReject>,
    ) {
        let type_id = TypeId::of::<rfc003::AcceptResponse<SL, TL>>();
        let mut pending_requests = self.pending_requests.lock().unwrap();
        pending_requests
            .remove(&type_id)
            .unwrap()
            .send(Box::new(response))
            .unwrap()
    }
}

impl Client for FakeClient {
    fn send_swap_request<
        SL: Ledger,
        TL: Ledger,
        SA: Into<wire_types::Asset>,
        TA: Into<wire_types::Asset>,
    >(
        &self,
        _request: rfc003::Request<SL, TL, SA, TA>,
    ) -> Box<
        Future<
                Item = Result<rfc003::AcceptResponse<SL, TL>, SwapReject>,
                Error = transport_protocol::client::Error<json::Frame>,
            > + Send,
    > {
        let type_id = TypeId::of::<rfc003::AcceptResponse<SL, TL>>();
        let (sender, receiver) = oneshot::channel::<Box<Any + Send>>();

        {
            self.pending_requests
                .lock()
                .unwrap()
                .insert(type_id, sender);
        }

        Box::new(receiver.map_err(|_| unimplemented!()).map(|response| {
            let _any: &(Any + Send) = response.borrow();
            _any.downcast_ref::<Result<rfc003::AcceptResponse<SL, TL>, SwapReject>>()
                .unwrap()
                .to_owned()
        }))
    }
}
