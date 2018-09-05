use super::client::ApiClient;
use api_client::{
    client::{RedeemDetails, TradingServiceError},
    BuyOfferRequestBody, BuyOrderRequestBody, OfferResponseBody, RequestToFund, TradeId,
};
use bitcoin_rpc_client;
use bitcoin_support::BitcoinQuantity;
use common_types;
use ethereum_support::{self, EthereumQuantity};
use offer::Symbol;
use std::str::FromStr;
use uuid::Uuid;

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
            sell_amount: BitcoinQuantity::from_bitcoin(7.0),
            buy_amount: EthereumQuantity::from_eth(100.0),
        })
    }
    fn request_order(
        &self,
        _symbol: &Symbol,
        _uid: Uuid,
        _request: &BuyOrderRequestBody,
    ) -> Result<RequestToFund, TradingServiceError> {
        Ok(RequestToFund {
            address_to_fund: bitcoin_rpc_client::Address::from_str(
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
            address: ethereum_support::Address::from_str(
                "00a329c0648769a73afac7f9381e08fb43dbea72",
            ).unwrap(),
            data: common_types::secret::Secret::from_str(
                "1234567890123456789012345678901212345678901234567890123456789012",
            ).unwrap(),
            gas: 20_000,
        })
    }
}
