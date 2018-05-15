#[derive(Serialize, Deserialize)]
pub struct Symbol(pub String); // Expected format: BTC:LTC

#[derive(Serialize, Deserialize)]
pub struct OfferRequest {
    symbol: Symbol,
    sell_amount: u32,
}

#[derive(Serialize, Deserialize)]
pub struct Offer {
    pub symbol: Symbol,
    pub rate: f32,
    pub uid: String,
}

#[derive(Clone)]
pub struct ExchangeApiUrl(pub String);
