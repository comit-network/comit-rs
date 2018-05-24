use bitcoin_rpc::Address;
use exchange_api_client::Offer;
use std::collections::HashMap;
use std::sync::Mutex;
use symbol::Symbol;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OfferCreated {
    pub trade_id: Uuid,
    pub symbol: Symbol,
    pub rate: f32,
    pub address: Address,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TradeCreated {
    pub trade_id: Uuid,
    pub symbol: Symbol,
    pub rate: f32,
    pub address: Address,
}

impl From<Offer> for OfferCreated {
    fn from(offer: Offer) -> Self {
        OfferCreated {
            trade_id: offer.uid,
            symbol: offer.symbol,
            rate: offer.rate,
            address: offer.address,
        }
    }
}

pub struct EventStore {
    offer_created_events: Mutex<HashMap<Uuid, OfferCreated>>,
    trade_created_events: Mutex<HashMap<Uuid, TradeCreated>>,
}

impl EventStore {
    pub fn new() -> EventStore {
        EventStore {
            offer_created_events: Mutex::new(HashMap::new()),
            trade_created_events: Mutex::new(HashMap::new()),
        }
    }

    pub fn store_offer_created(&self, event: OfferCreated) {
        let mut offers = self.offer_created_events.lock().unwrap();
        let uid = event.trade_id.clone();

        offers.insert(uid, event.clone());
    }

    pub fn store_trade_created(&self, event: TradeCreated) {
        let mut offers = self.trade_created_events.lock().unwrap();
        let uid = event.trade_id.clone();

        offers.insert(uid, event.clone());
    }
}
