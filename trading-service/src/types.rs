#[derive(Serialize, Deserialize)]
pub struct Symbol(String); // Expected format: BTC:LTC

#[derive(Serialize, Deserialize)]
pub struct OfferRequest {
    symbol: Symbol,
    sell_amount: u32,
}

#[derive(Serialize, Deserialize)]
pub struct Offer {
    symbol: Symbol,
    rate: f32,
    uid: String,
}

#[derive(Clone)]
pub struct ExchangeApiUrl(pub String);
