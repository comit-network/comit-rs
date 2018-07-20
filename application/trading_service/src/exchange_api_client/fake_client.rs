use super::client::ApiClient;
use bitcoin_rpc::Address;
use bitcoin_support::BitcoinQuantity;
use ethereum_support::EthereumQuantity;
use event_store::TradeId;
use exchange_api_client::client::{OfferResponseBody, OrderRequestBody, OrderResponseBody};
use reqwest;
use std::str::FromStr;
use symbol::Symbol;
use uuid::Uuid;

#[allow(dead_code)]
pub struct FakeApiClient;

impl ApiClient for FakeApiClient {
    fn create_offer(
        &self,
        symbol: Symbol,
        _amount: f64,
    ) -> Result<OfferResponseBody, reqwest::Error> {
        let offer = OfferResponseBody {
            uid: TradeId::from(Uuid::new_v4()),
            symbol: symbol.clone(),
            rate: 0.42,
            btc_amount: BitcoinQuantity::from_satoshi(24),
            eth_amount: EthereumQuantity::from_eth(10.0),
        };
        Ok(offer)
    }

    fn create_order(
        &self,
        _symbol: Symbol,
        _uid: TradeId,
        _trade_request: &OrderRequestBody,
    ) -> Result<OrderResponseBody, reqwest::Error> {
        let accept = OrderResponseBody {
            exchange_refund_address: "34b19d15e793883d840c563d7dbc8a6723465146".into(),
            exchange_contract_time_lock: 43200,
            exchange_success_address: Address::from_str(
                "bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap",
            ).unwrap(),
        };

        Ok(accept)
    }
}
