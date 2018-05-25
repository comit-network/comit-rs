use bitcoin_rpc;
use exchange_api_client::Offer;
use secret::Secret;
use std::collections::HashMap;
use std::sync::RwLock;
use stub::BtcBlockHeight;
use stub::BtcHtlc;
use stub::{EthAddress, EthTimeDelta};
use symbol::Symbol;
use uuid::Uuid;

//pub use self::OfferCreated as OfferCreatedState;

// State after exchange has made an offer
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OfferCreated {
    pub uid: Uuid,
    pub symbol: Symbol,
    pub rate: f32, // Actually need to specify the exact amounts of each currency
    pub exchange_success_address: bitcoin_rpc::Address,
}

impl From<Offer> for OfferCreated {
    fn from(offer: Offer) -> Self {
        OfferCreated {
            uid: offer.uid,
            symbol: offer.symbol,
            rate: offer.rate,
            exchange_success_address: offer.exchange_success_address,
        }
    }
}

// State after client accepts trade offer
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TradeCreated {
    pub uid: Uuid,
    pub client_success_address: EthAddress,
    pub client_refund_address: bitcoin_rpc::Address,
    pub secret: Secret,
    pub long_relative_timelock: BtcBlockHeight,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TradeAccepted {
    pub uid: Uuid,
    pub exchange_refund_address: EthAddress,
    pub short_relative_timelock: EthTimeDelta,
    pub htlc: BtcHtlc,
}

pub struct EventStore {
    offer_created_events: RwLock<HashMap<Uuid, OfferCreated>>,
    trade_created_events: RwLock<HashMap<Uuid, TradeCreated>>,
    trade_accepted_events: RwLock<HashMap<Uuid, TradeAccepted>>,
}

impl EventStore {
    pub fn new() -> EventStore {
        EventStore {
            offer_created_events: RwLock::new(HashMap::new()),
            trade_created_events: RwLock::new(HashMap::new()),
            trade_accepted_events: RwLock::new(HashMap::new()),
        }
    }

    pub fn store_offer_created(&self, event: OfferCreated) {
        let mut offer_created_events = self.offer_created_events.write().unwrap();
        let uid = event.uid.clone();
        offer_created_events.insert(uid, event);
    }

    pub fn store_trade_created(&self, event: TradeCreated) {
        let mut trade_created_events = self.trade_created_events.write().unwrap();
        let uid = event.uid.clone();
        trade_created_events.insert(uid, event);
    }

    pub fn store_trade_accepted(&self, event: TradeAccepted) {
        let mut trade_acceptances = self.trade_accepted_events.write().unwrap();
        let uid = event.uid.clone();
        trade_acceptances.insert(uid, event);
    }

    pub fn get_offer_created(&self, id: &Uuid) -> Option<OfferCreated> {
        let offers = self.offer_created_events.read().unwrap();
        offers.get(id).map(|offer| offer.clone())
    }

    pub fn get_trade_created(&self, id: &Uuid) -> Option<TradeCreated> {
        let trades = self.trade_created_events.read().unwrap();
        trades.get(id).map(|trade| trade.clone())
    }

    pub fn get_trade_accept(&self, id: &Uuid) -> Option<TradeAccepted> {
        let trade_acceptances = self.trade_accepted_events.read().unwrap();
        trade_acceptances
            .get(id)
            .map(|trade_accept| trade_accept.clone())
    }
}
