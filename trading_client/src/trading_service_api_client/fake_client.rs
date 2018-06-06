use super::client::ApiClient;
use reqwest;
use std::str::FromStr;
use trading_service_api_client::BuyOfferRequestBody;
use trading_service_api_client::OfferResponseBody;
use trading_service_api_client::client::RedeemDetails;
use uuid::Uuid;
use web3::types::Address as EthAddress;

use offer::Symbol;

#[allow(dead_code)]
pub struct FakeApiClient;

impl ApiClient for FakeApiClient {
    fn request_offer(
        &self,
        symbol: Symbol,
        _offer_request: &BuyOfferRequestBody,
    ) -> Result<OfferResponseBody, reqwest::Error> {
        Ok(OfferResponseBody {
            uid: Uuid::from_str("a83aac12-0c78-417e-88e4-1a2948c6d538").unwrap(),
            symbol: symbol,
            rate: 0.6876231,
        })
    }
    fn request_redeem_details(&self, _uid: Uuid) -> Result<RedeemDetails, reqwest::Error> {
        Ok(RedeemDetails {
            uid: Uuid::new_v4(),
            address: EthAddress::from_str("00a329c0648769a73afac7f9381e08fb43dbea72").unwrap(),
            gas: 20_000,
        })
    }
}
