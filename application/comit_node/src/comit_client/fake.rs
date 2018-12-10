use comit_client::{
    rfc003, Client, ClientFactory, ClientFactoryError, SwapReject, SwapResponseError,
};
use futures::{
    sync::oneshot::{self, Sender},
    Future,
};
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use swap_protocols::{self, asset::Asset};

#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct FakeClient {
    pending_requests: Mutex<HashMap<TypeId, Sender<Box<Any + Send>>>>,
}

impl FakeClient {
    pub fn resolve_request<
        AL: swap_protocols::rfc003::Ledger,
        BL: swap_protocols::rfc003::Ledger,
    >(
        &self,
        response: Result<rfc003::AcceptResponseBody<AL, BL>, SwapReject>,
    ) {
        let type_id = TypeId::of::<rfc003::AcceptResponseBody<AL, BL>>();
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
        AL: swap_protocols::rfc003::Ledger,
        BL: swap_protocols::rfc003::Ledger,
        AA: Asset,
        BA: Asset,
    >(
        &self,
        _request: rfc003::Request<AL, BL, AA, BA>,
    ) -> Box<
        Future<
                Item = Result<rfc003::AcceptResponseBody<AL, BL>, SwapReject>,
                Error = SwapResponseError,
            > + Send,
    > {
        let type_id = TypeId::of::<rfc003::AcceptResponseBody<AL, BL>>();
        let (sender, receiver) = oneshot::channel::<Box<Any + Send>>();

        {
            self.pending_requests
                .lock()
                .unwrap()
                .insert(type_id, sender);
        }

        Box::new(receiver.map_err(|_| unimplemented!()).map(|response| {
            use std::borrow::Borrow;
            let _any: &(Any + Send) = response.borrow();
            _any.downcast_ref::<Result<rfc003::AcceptResponseBody<AL, BL>, SwapReject>>()
                .unwrap()
                .to_owned()
        }))
    }
}

#[derive(Debug, Default)]
pub struct FakeClientFactory {
    pub fake_client: Arc<FakeClient>,
}

impl FakeClientFactory {
    pub fn fake_client(&self) -> &FakeClient {
        &self.fake_client
    }
}

impl ClientFactory<FakeClient> for FakeClientFactory {
    fn client_for(
        &self,
        _comit_node_socket_addr: SocketAddr,
    ) -> Result<Arc<FakeClient>, ClientFactoryError> {
        Ok(self.fake_client.clone())
    }
}
