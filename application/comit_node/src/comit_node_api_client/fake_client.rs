use bitcoin_support::{self, BitcoinQuantity};
use comit_node_api_client::{ApiClient, SwapRequestError};
use common_types::seconds::Seconds;
use ethereum_support::{self, EthereumQuantity};
use ganp::{
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum},
    rfc003,
};
use std::str::FromStr;

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
        _swap_request: rfc003::Request<Bitcoin, Ethereum, BitcoinQuantity, EthereumQuantity>,
    ) -> Result<rfc003::AcceptResponse<Bitcoin, Ethereum>, SwapRequestError> {
        let accept = rfc003::AcceptResponse {
            target_ledger_refund_identity: ethereum_support::Address::from_str(
                "34b19d15e793883d840c563d7dbc8a6723465146",
            ).unwrap(),
            target_ledger_lock_duration: Seconds::new(43200),
            source_ledger_success_identity: bitcoin_support::Address::from_str(
                "bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap",
            ).unwrap()
                .into(),
        };

        Ok(accept)
    }

    fn create_sell_order(
        &self,
        _swap_request: rfc003::Request<Ethereum, Bitcoin, EthereumQuantity, BitcoinQuantity>,
    ) -> Result<rfc003::AcceptResponse<Ethereum, Bitcoin>, SwapRequestError> {
        let accept = rfc003::AcceptResponse {
            target_ledger_refund_identity: bitcoin_support::Address::from_str(
                "bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap",
            ).unwrap()
                .into(),
            target_ledger_lock_duration: 43200.into(),
            source_ledger_success_identity: ethereum_support::Address::from_str(
                "34b19d15e793883d840c563d7dbc8a6723465146",
            ).unwrap(),
        };

        Ok(accept)
    }
}
