use super::client::ApiClient;
use bitcoin_rpc::Address;
use exchange_api_client::client::{OfferResponseBody, OrderRequestBody, OrderResponseBody};
use reqwest;
use stub::EthAddress;
use stub::EthTimeDelta;
use symbol::Symbol;
use uuid::Uuid;

#[allow(dead_code)]
pub struct FakeApiClient;

impl ApiClient for FakeApiClient {
    fn create_offer(
        &self,
        symbol: Symbol,
        _amount: u32,
    ) -> Result<OfferResponseBody, reqwest::Error> {
        let offer = OfferResponseBody {
            uid: Uuid::new_v4(),
            symbol: symbol.clone(),
            rate: 0.42,
        };
        Ok(offer)
    }

    fn create_order(
        &self,
        _symbol: Symbol,
        uid: Uuid,
        _trade_request: &OrderRequestBody,
    ) -> Result<OrderResponseBody, reqwest::Error> {
        let accept = OrderResponseBody {
            uid: uid,
            exchange_refund_address: EthAddress(
                "0x34b19d15e793883d840c563d7dbc8a6723465146".to_string(),
            ),
            exchange_success_address: Address::from("bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap"),
            short_relative_timelock: EthTimeDelta(43200),
        };

        Ok(accept)
    }
}
