use bitcoin_support::{self, BitcoinQuantity};
use comit_client::{Client, SwapReject};
use common_types::seconds::Seconds;
use ethereum_support::{self, EthereumQuantity};
use futures::{future, Future};
use ganp::{ledger::Ledger, rfc003, swap};
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    str::FromStr,
};
use transport_protocol::{self, json};

// #[allow(dead_code)]
// pub struct FakeClient {
//     pending_requests: HashMap<TypeId, Sender>;
// }

pub struct FakeClient {}

impl FakeClient {
    pub fn new() -> Self {
        FakeClient {}
    }
}

impl Client for FakeClient {
    fn send_swap_request<SL: Ledger, TL: Ledger, SA: Into<swap::Asset>, TA: Into<swap::Asset>>(
        &self,
        request: rfc003::Request<SL, TL, SA, TA>,
    ) -> Box<
        Future<
            Item = Result<rfc003::AcceptResponse<SL, TL>, SwapReject>,
            Error = transport_protocol::client::Error<json::Frame>,
        >,
    > {
        Box::new(future::ok(Ok(rfc003::AcceptResponse {
            target_ledger_refund_identity: Default::default(),
            target_ledger_lock_duration: Default::default(),
            source_ledger_success_identity: Default::default(),
        })))
    }
}
