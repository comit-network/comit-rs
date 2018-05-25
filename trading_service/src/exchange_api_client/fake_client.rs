use super::client::ApiClient;
use bitcoin_rpc::Address;
use exchange_api_client::client::TradeAcceptance;
use exchange_api_client::client::{Offer, TradeRequestBody};
use reqwest;
use stub::EthAddress;
use stub::EthTimeDelta;
use symbol::Symbol;
use uuid::Uuid;

#[allow(dead_code)]
pub struct FakeApiClient;

impl ApiClient for FakeApiClient {
    fn create_offer(&self, symbol: Symbol, amount: u32) -> Result<Offer, reqwest::Error> {
        let offer = Offer {
            uid: Uuid::new_v4(),
            symbol: symbol.clone(),
            rate: 0.42,
            exchange_success_address: Address::from("mtgyGsXBNG7Yta5rcMgWH4x9oGE5rm3ty9"),
        };
        Ok(offer)
    }

    fn create_trade(
        &self,
        symbol: Symbol,
        trade_request: &TradeRequestBody,
    ) -> Result<TradeAcceptance, reqwest::Error> {
        let accept = TradeAcceptance {
            uid: trade_request.uid,
            exchange_refund_address: EthAddress(
                "0x34b19d15e793883d840c563d7dbc8a6723465146".to_string(),
            ),
            short_relative_timelock: EthTimeDelta(43200),
        };

        Ok(accept)
    }
}
