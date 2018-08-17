use common_types::TradingSymbol;
use exchange_api_client::{
    client::OrderResponseBody, ApiClient, OfferResponseBody, OrderRequestBody,
};
use reqwest;
use swaps::TradeId;
use uuid::Uuid;

#[allow(dead_code)]
pub struct FakeApiClient;

impl FakeApiClient {
    pub fn new() -> Self {
        FakeApiClient {}
    }
}

impl ApiClient for FakeApiClient {
    fn create_buy_offer(
        &self,
        symbol: TradingSymbol,
        _amount: f64,
    ) -> Result<OfferResponseBody, reqwest::Error> {
        let offer = OfferResponseBody {
            uid: TradeId::from(Uuid::new_v4()),
            symbol,
            rate: 0.42,
            sell_amount: String::from("24"),
            buy_amount: String::from("10.0"),
        };
        Ok(offer)
    }

    fn create_buy_order(
        &self,
        _symbol: TradingSymbol,
        _uid: TradeId,
        _trade_request: &OrderRequestBody,
    ) -> Result<OrderResponseBody, reqwest::Error> {
        let accept = OrderResponseBody {
            exchange_refund_address: String::from("34b19d15e793883d840c563d7dbc8a6723465146"),
            exchange_contract_time_lock: 43200,
            exchange_success_address: String::from("bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap"),
        };

        Ok(accept)
    }
}
