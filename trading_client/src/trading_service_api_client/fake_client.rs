use super::client::ApiClient;
use reqwest;
use std::str::FromStr;
use trading_service_api_client::client::RedeemDetails;
use types::Offer;
use types::OfferRequest;
use uuid::Uuid;
use web3::types::Address as EthAddress;

#[allow(dead_code)]
pub struct FakeApiClient;

impl ApiClient for FakeApiClient {
    fn request_offer(&self, _offer_request: &OfferRequest) -> Result<Offer, reqwest::Error> {
        unimplemented!()
    }
    fn request_redeem_details(&self, _uid: Uuid) -> Result<RedeemDetails, reqwest::Error> {
        Ok(RedeemDetails {
            uid: Uuid::new_v4(),
            address: EthAddress::from_str("00a329c0648769a73afac7f9381e08fb43dbea72").unwrap(),
            gas: 20_000,
        })
    }
}
