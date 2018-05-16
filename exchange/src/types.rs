use bitcoin_rpc::Address;
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Symbol(pub String); // Expected format: BTC:LTC

#[derive(Debug, Deserialize)]
pub struct Rate {
    pub symbol: Symbol,
    pub rate: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OfferRequest {
    pub symbol: Symbol,
    pub amount: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Offer {
    pub symbol: Symbol,
    pub rate: f32,
    pub uid: Uuid,
    pub address: Address,
}

pub struct Offers {
    pub all_offers: Mutex<HashMap<Uuid, Offer>>,
}

impl Offers {
    pub fn new() -> Offers {
        Offers {
            all_offers: Mutex::new(HashMap::new()),
        }
    }
}

#[derive(Clone)]
pub struct TreasuryApiUrl(pub String);
