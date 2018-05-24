use super::client::ApiClient;
use bitcoin_rpc::Address;
use exchange_api_client::client::Offer;
use reqwest;
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
            address: Address::from("mtgyGsXBNG7Yta5rcMgWH4x9oGE5rm3ty9"),
        };
        Ok(offer)
    }
}
