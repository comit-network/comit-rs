use super::client::ApiClient;
use bitcoin_rpc;
use offer::Symbol;
use reqwest;
use std::str::FromStr;
use trading_service_api_client::BuyOfferRequestBody;
use trading_service_api_client::BuyOrderRequestBody;
use trading_service_api_client::OfferResponseBody;
use trading_service_api_client::RequestToFund;
use trading_service_api_client::client::RedeemDetails;
use uuid::Uuid;
use web3::types::Address as EthAddress;

#[allow(dead_code)]
pub struct FakeApiClient;

impl ApiClient for FakeApiClient {
    fn request_offer(
        &self,
        symbol: &Symbol,
        _offer_request: &BuyOfferRequestBody,
    ) -> Result<OfferResponseBody, reqwest::Error> {
        let symbol: Symbol = symbol.clone();
        Ok(OfferResponseBody {
            uid: Uuid::from_str("a83aac12-0c78-417e-88e4-1a2948c6d538").unwrap(),
            symbol: symbol,
            rate: 0.6876231,
        })
    }
    fn request_order(
        &self,
        _symbol: &Symbol,
        _uid: Uuid,
        _request: &BuyOrderRequestBody,
    ) -> Result<RequestToFund, reqwest::Error> {
        Ok(RequestToFund {
            address_to_fund: bitcoin_rpc::Address::from(
                "bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap",
            ),
        })
    }
    fn request_redeem_details(
        &self,
        _symbol: Symbol,
        _uid: Uuid,
    ) -> Result<RedeemDetails, reqwest::Error> {
        Ok(RedeemDetails {
            uid: Uuid::new_v4(),
            address: EthAddress::from_str("00a329c0648769a73afac7f9381e08fb43dbea72").unwrap(),
            gas: 20_000,
        })
    }
}
