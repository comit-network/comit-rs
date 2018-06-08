use super::client::ApiClient;
use bitcoin_rpc;
use common_types;
use offer::Symbol;
use reqwest;
use std::str::FromStr;
use trading_service_api_client::BuyOfferRequestBody;
use trading_service_api_client::BuyOrderRequestBody;
use trading_service_api_client::OfferResponseBody;
use trading_service_api_client::RequestToFund;
use trading_service_api_client::TradeId;
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
            uid: TradeId::from_str("a83aac12-0c78-417e-88e4-1a2948c6d538").unwrap(),
            symbol: symbol,
            amount: 100,
            rate: 0.6876231,
            sell_amount: 145,
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
            sell_amount: 1001,
        })
    }
    fn request_redeem_details(
        &self,
        _symbol: Symbol,
        _uid: Uuid,
    ) -> Result<RedeemDetails, reqwest::Error> {
        Ok(RedeemDetails {
            address: EthAddress::from_str("00a329c0648769a73afac7f9381e08fb43dbea72").unwrap(),
            data: common_types::secret::Secret::from_str(
                "1234567890123456789012345678901212345678901234567890123456789012",
            ).unwrap(),
            gas: 20_000,
        })
    }
}
