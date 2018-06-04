use std::str::FromStr;

#[derive(Debug, Serialize)]
pub struct Currency(String);

#[derive(Serialize)]
pub struct OfferRequest {
    sell: Currency,
    buy: Currency,
    sell_amount: u32,
}

#[derive(Serialize, Deserialize)]
pub struct Offer {
    symbol: String,
    rate: f32,
    uid: String,
}

impl FromStr for Currency {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        Ok(Currency(s.to_string()))
    }
}

#[derive(Clone)]
pub struct TradingApiUrl(pub String);
