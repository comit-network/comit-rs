pub mod mocks;

pub use self::mocks::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OfferResponseBody {
    pub uid: String,
    pub symbol: String,
    pub rate: f64,
    pub buy_amount: String,
    pub sell_amount: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RequestToFund {
    pub address_to_fund: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RedeemDetails {
    pub address: String,
    pub data: String,
    pub gas: u64,
}
