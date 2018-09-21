use bitcoin_support;
use comit_node_api_client::{
    client::{OrderResponseBody, SwapRequestError},
    ApiClient, OrderRequestBody,
};
use common_types::seconds::Seconds;
use ethereum_support;
use std::str::FromStr;
use swap_protocols::ledger::{bitcoin::Bitcoin, ethereum::Ethereum};

#[allow(dead_code)]
pub struct FakeApiClient;

impl FakeApiClient {
    pub fn new() -> Self {
        FakeApiClient {}
    }
}

impl ApiClient for FakeApiClient {
    fn create_buy_order(
        &self,
        _trade_request: &OrderRequestBody<Ethereum, Bitcoin>,
    ) -> Result<OrderResponseBody<Ethereum, Bitcoin>, SwapRequestError> {
        let accept = OrderResponseBody {
            bob_refund_address: ethereum_support::Address::from_str(
                "34b19d15e793883d840c563d7dbc8a6723465146",
            ).unwrap(),
            bob_contract_time_lock: Seconds::new(43200),
            bob_success_address: bitcoin_support::Address::from_str(
                "bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap",
            ).unwrap(),
        };

        Ok(accept)
    }

    fn create_sell_order(
        &self,
        _trade_request: &OrderRequestBody<Bitcoin, Ethereum>,
    ) -> Result<OrderResponseBody<Bitcoin, Ethereum>, SwapRequestError> {
        let accept = OrderResponseBody {
            bob_refund_address: bitcoin_support::Address::from_str(
                "bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap",
            ).unwrap(),
            bob_contract_time_lock: 43200.into(),
            bob_success_address: ethereum_support::Address::from_str(
                "34b19d15e793883d840c563d7dbc8a6723465146",
            ).unwrap(),
        };

        Ok(accept)
    }
}
