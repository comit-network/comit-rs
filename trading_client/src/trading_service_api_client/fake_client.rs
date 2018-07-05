use super::client::ApiClient;
use bitcoin_rpc;
use common_types;
use common_types::{BitcoinQuantity, EthereumQuantity};
use offer::Symbol;
use std::str::FromStr;
use trading_service_api_client::BuyOfferRequestBody;
use trading_service_api_client::BuyOrderRequestBody;
use trading_service_api_client::OfferResponseBody;
use trading_service_api_client::RequestToFund;
use trading_service_api_client::TradeId;
use trading_service_api_client::client::RedeemDetails;
use trading_service_api_client::client::TradingServiceError;
use uuid::Uuid;
use web3::types::Address as EthAddress;

#[allow(dead_code)]
pub struct FakeApiClient;

impl ApiClient for FakeApiClient {
    fn request_offer(
        &self,
        symbol: &Symbol,
        _offer_request: &BuyOfferRequestBody,
    ) -> Result<OfferResponseBody, TradingServiceError> {
        let symbol: Symbol = symbol.clone();
        Ok(OfferResponseBody {
            uid: TradeId::from_str("a83aac12-0c78-417e-88e4-1a2948c6d538").unwrap(),
            symbol,
            rate: 0.07,
            btc_amount: BitcoinQuantity::from_bitcoin(7.0),
            eth_amount: EthereumQuantity::from_eth(100.0),
        })
    }
    fn request_order(
        &self,
        _symbol: &Symbol,
        _uid: Uuid,
        _request: &BuyOrderRequestBody,
    ) -> Result<RequestToFund, TradingServiceError> {
        Ok(RequestToFund {
            address_to_fund: bitcoin_rpc::RpcAddress::from_str(
                "bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap",
            ).unwrap(),
            btc_amount: BitcoinQuantity::from_bitcoin(1001.0),
            eth_amount: EthereumQuantity::from_eth(140.0),
        })
    }
    fn request_redeem_details(
        &self,
        _symbol: Symbol,
        _uid: Uuid,
    ) -> Result<RedeemDetails, TradingServiceError> {
        Ok(RedeemDetails {
            address: EthAddress::from_str("00a329c0648769a73afac7f9381e08fb43dbea72").unwrap(),
            data: common_types::secret::Secret::from_str(
                "1234567890123456789012345678901212345678901234567890123456789012",
            ).unwrap(),
            gas: 20_000,
        })
    }
}
