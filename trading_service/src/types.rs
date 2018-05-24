use bitcoin_rpc::Address;
use secret::{Secret, SecretHash};
use std::collections::HashMap;
use std::fmt;
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Symbol(pub String); // Expected format: BTC-LTC

#[derive(Serialize, Deserialize)]
pub struct OfferRequestBody {
    pub amount: u32,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Epoch(pub u32); // Unix timestamp

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl OfferRequestBody {
    pub fn new(offer_request: &OfferRequest) -> OfferRequestBody {
        let amount = offer_request.amount;
        OfferRequestBody { amount }
    }
}

#[derive(Serialize, Deserialize)]
pub struct OfferRequest {
    pub symbol: Symbol,
    pub amount: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Offer {
    pub uid: Uuid,
    pub symbol: Symbol,
    pub rate: f32,
    pub address: Address,
    pub expiry: Epoch,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SwapProposal {
    pub uid: Uuid,
    pub symbol: Symbol,
    pub rate: f32,
    pub address: Address,
    pub expiry: Epoch,
    pub secret_hash: SecretHash,
}

pub struct SwapData {
    offer: SwapProposal,
    secret: Secret,
}

impl SwapProposal {
    pub fn new(
        uid: Uuid,
        symbol: Symbol,
        rate: f32,
        address: Address,
        expiry: Epoch,
        secret_hash: SecretHash,
    ) -> SwapProposal {
        SwapProposal {
            uid,
            symbol,
            rate,
            address,
            secret_hash,
            expiry,
        }
    }

    pub fn from_exchange_offer(exchange_offer: Offer, secret_hash: SecretHash) -> SwapProposal {
        SwapProposal::new(
            exchange_offer.uid,
            exchange_offer.symbol,
            exchange_offer.rate,
            exchange_offer.address,
            exchange_offer.expiry,
            secret_hash,
        )
    }
}

impl SwapData {
    pub fn new(offer: SwapProposal, secret: Secret) -> SwapData {
        SwapData { offer, secret }
    }

    pub fn uid(&self) -> Uuid {
        self.offer.uid
    }
}

#[derive(Clone)]
pub struct ExchangeApiUrl(pub String);

pub struct Offers {
    pub all_offers: Mutex<HashMap<Uuid, SwapData>>,
}

impl Offers {
    pub fn new() -> Offers {
        Offers {
            all_offers: Mutex::new(HashMap::new()),
        }
    }
}
