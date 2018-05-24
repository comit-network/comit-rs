use bitcoin_rpc::Address;
use exchange_api_client::Offer;
use std::collections::HashMap;
use std::sync::RwLock;
use symbol::Symbol;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OfferCreated {
    pub trade_id: Uuid,
    pub symbol: Symbol,
    pub rate: f32,
    pub exchange_success_address: Address,
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
            exchange_success_address: offer.exchange_success_address,
        }
    }
}

pub struct EventStore {
    offer_created_events: RwLock<HashMap<Uuid, OfferCreated>>,
    trade_created_events: RwLock<HashMap<Uuid, TradeCreated>>,
}

impl EventStore {
    pub fn new() -> EventStore {
        EventStore {
            offer_created_events: RwLock::new(HashMap::new()),
            trade_created_events: RwLock::new(HashMap::new()),
        }
    }

    pub fn store_offer_created(&self, event: OfferCreated) {
        let mut offers = self.offer_created_events.write().unwrap();
        let uid = event.trade_id.clone();

        offers.insert(uid, event.clone());
    }

    pub fn store_trade_created(&self, event: TradeCreated) {
        let mut offers = self.trade_created_events.write().unwrap();
        let uid = event.trade_id.clone();

        offers.insert(uid, event.clone());
    }

    pub fn get_offer_created(&self, id: &Uuid) -> Option<OfferCreated> {
        let offers = self.offer_created_events.read().unwrap();
        offers.get(id).map(|offer| offer.clone())
    }

    pub fn get_trade_created(&self, id: &Uuid) -> Option<TradeCreated> {
        let trades = self.trade_created_events.read().unwrap();
        trades.get(id).map(|trade| trade.clone())
    }
}
