use bitcoin_rpc;
use common_types;
use common_types::{BitcoinQuantity, EthereumQuantity};
use offer::Symbol;
use regex::Regex;
use reqwest;
use std::fmt;
use std::str::FromStr;
use uuid::ParseError;
use uuid::Uuid;
use web3::types::Address as EthAddress;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TradeId(Uuid);

impl FromStr for TradeId {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        let uid = Uuid::from_str(s)?;
        Ok(TradeId(uid))
    }
}

impl fmt::Display for TradeId {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.0.fmt(f)
    }
}

#[derive(Clone)]
pub struct TradingApiUrl(pub String);

#[allow(dead_code)]
pub struct DefaultApiClient {
    pub url: TradingApiUrl,
    pub client: reqwest::Client,
}

#[derive(Deserialize, Serialize)]
pub struct BuyOfferRequestBody {
    amount: u64,
}

impl BuyOfferRequestBody {
    pub fn new(amount: u64) -> BuyOfferRequestBody {
        BuyOfferRequestBody { amount }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OfferResponseBody {
    pub uid: TradeId,
    pub symbol: Symbol,
    pub rate: f64,
    //TODO: trading-cli should be agnostic of the currencies
    pub eth_amount: EthereumQuantity,
    pub btc_amount: BitcoinQuantity,
}

#[derive(Deserialize, Serialize)]
pub struct BuyOrderRequestBody {
    client_success_address: EthAddress,
    client_refund_address: bitcoin_rpc::Address,
}

impl BuyOrderRequestBody {
    pub fn new(
        client_success_address: String,
        client_refund_address: String,
    ) -> BuyOrderRequestBody {
        let client_success_address = client_success_address.clone();

        let re = Regex::new("^0x").unwrap();
        let client_success_address = re.replace(&client_success_address.as_str(), "");

        let client_success_address = EthAddress::from_str(&client_success_address)
            .expect("Could not convert the success address");
        let client_refund_address = bitcoin_rpc::Address::from(client_refund_address.as_str());

        BuyOrderRequestBody {
            client_success_address,
            client_refund_address,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RequestToFund {
    pub address_to_fund: bitcoin_rpc::Address,
    pub btc_amount: BitcoinQuantity,
    pub eth_amount: EthereumQuantity,
}

#[derive(Deserialize)]
pub struct RedeemDetails {
    pub address: EthAddress,
    pub data: common_types::secret::Secret,
    pub gas: u64,
}

#[derive(Debug)]
pub enum TradingServiceError {
    OfferAborted(reqwest::Error),
    OrderAborted(reqwest::Error),
    RedeemAborted(reqwest::Error),
}

pub trait ApiClient {
    fn request_offer(
        &self,
        symbol: &Symbol,
        offer_request: &BuyOfferRequestBody,
    ) -> Result<OfferResponseBody, TradingServiceError>;

    fn request_order(
        &self,
        symbol: &Symbol,
        uid: Uuid,
        request: &BuyOrderRequestBody,
    ) -> Result<RequestToFund, TradingServiceError>;

    fn request_redeem_details(
        &self,
        symbol: Symbol,
        uid: Uuid,
    ) -> Result<RedeemDetails, TradingServiceError>;
}

impl ApiClient for DefaultApiClient {
    fn request_offer(
        &self,
        symbol: &Symbol,
        request: &BuyOfferRequestBody,
    ) -> Result<OfferResponseBody, TradingServiceError> {
        let client = reqwest::Client::new();
        client
            .post(format!("{}/trades/{}/buy-offers", self.url.0, symbol).as_str())
            .json(request)
            .send()
            .and_then(|mut res| res.json::<OfferResponseBody>())
            .map_err(|err| TradingServiceError::OfferAborted(err))
    }

    fn request_order(
        &self,
        symbol: &Symbol,
        uid: Uuid,
        request: &BuyOrderRequestBody,
    ) -> Result<RequestToFund, TradingServiceError> {
        let client = reqwest::Client::new();
        client
            .post(format!("{}/trades/{}/{}/buy-orders", self.url.0, symbol, uid).as_str())
            .json(request)
            .send()
            .and_then(|mut res| res.json::<RequestToFund>())
            .map_err(|err| TradingServiceError::OrderAborted(err))
    }

    fn request_redeem_details(
        &self,
        symbol: Symbol,
        uid: Uuid,
    ) -> Result<RedeemDetails, TradingServiceError> {
        let client = reqwest::Client::new();
        client
            .get(format!("{}/trades/{}/{}/redeem-orders", self.url.0, symbol, uid).as_str())
            .send()
            .and_then(|mut res| res.json::<RedeemDetails>())
            .map_err(|err| TradingServiceError::RedeemAborted(err))
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn given_an_hex_address_with_0x_should_remove_0x() {
        let address = "0x00a329c0648769a73afac7f9381e08fb43dbea72".to_string();
        let refund_address = "bcrt1qryj6ya9vqpph8w65992nhk64cs890vfy0khsfg".to_string();
        let order_request_body = BuyOrderRequestBody::new(address, refund_address);

        let eth_address = EthAddress::from_str("00a329c0648769a73afac7f9381e08fb43dbea72").unwrap();
        assert_eq!(order_request_body.client_success_address, eth_address)
    }

    #[test]
    fn given_an_hex_address_without_0x_should_return_same_address() {
        let address = "00a329c0648769a73afac7f9381e08fb43dbea72".to_string();
        let refund_address = "bcrt1qryj6ya9vqpph8w65992nhk64cs890vfy0khsfg".to_string();
        let order_request_body = BuyOrderRequestBody::new(address, refund_address);

        let eth_address = EthAddress::from_str("00a329c0648769a73afac7f9381e08fb43dbea72").unwrap();
        assert_eq!(order_request_body.client_success_address, eth_address)
    }

}
