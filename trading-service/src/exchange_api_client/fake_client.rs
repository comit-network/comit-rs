use super::client::ApiClient;
use bitcoin_rpc::Address;
use reqwest;
use types::Offer;
use types::OfferRequest;
use uuid::Uuid;

#[allow(dead_code)]
pub struct FakeApiClient;

impl ApiClient for FakeApiClient {
    fn create_offer(&self, offer_request: &OfferRequest) -> Result<Offer, reqwest::Error> {
        let offer = Offer {
            uid: Uuid::new_v4(),
            symbol: offer_request.symbol.clone(),
            rate: 0.42,
            address: Address::from("mtgyGsXBNG7Yta5rcMgWH4x9oGE5rm3ty9"),
        };
        Ok(offer)
    }
}
