pub mod ganache_client;
pub mod mocks;

pub use self::ganache_client::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OfferResponseBody {
    pub uid: String,
    pub symbol: String,
    pub rate: f64,
    pub buy_amount: String,
    pub sell_amount: String,
}

impl PartialEq for OfferResponseBody {
    fn eq(&self, other: &OfferResponseBody) -> bool {
        self.symbol == other.symbol
            && self.rate == other.rate
            && self.buy_amount == other.buy_amount
            && self.sell_amount == other.sell_amount
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RequestToFund {
    pub address_to_fund: String,
    pub btc_amount: String,
    pub eth_amount: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RedeemDetails {
    pub address: String,
    pub data: String,
    pub gas: u64,
}
