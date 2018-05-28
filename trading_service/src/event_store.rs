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
}

impl From<Offer> for OfferCreated {
    fn from(offer: Offer) -> Self {
        OfferCreated {
            uid: offer.uid,
            symbol: offer.symbol,
            rate: offer.rate,
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
    // This is embedded in the HTLC but we keep it here as well for completeness
    pub exchange_success_address: bitcoin_rpc::Address,
    pub short_relative_timelock: EthTimeDelta,
    pub htlc: BtcHtlc,
}

pub struct EventStore {
    offer_created_events: RwLock<HashMap<Uuid, OfferCreated>>,
    trade_created_events: RwLock<HashMap<Uuid, TradeCreated>>,
    trade_accepted_events: RwLock<HashMap<Uuid, TradeAccepted>>,
}

#[derive(PartialEq)]
enum TradeState {
    NonExistent,
    OfferCreated,
    TradeCreated,
    TradeAccepted,
}

pub enum Error {
    IncorrectState,
}

impl EventStore {
    pub fn new() -> EventStore {
        EventStore {
            offer_created_events: RwLock::new(HashMap::new()),
            trade_created_events: RwLock::new(HashMap::new()),
            trade_accepted_events: RwLock::new(HashMap::new()),
        }
    }

    fn current_state(&self, id: &Uuid) -> TradeState {
        if self._get(&self.offer_created_events, id).is_none() {
            return TradeState::NonExistent;
        }

        if self._get(&self.trade_created_events, id).is_none() {
            return TradeState::OfferCreated;
        }

        if self._get(&self.trade_accepted_events, id).is_none() {
            return TradeState::TradeCreated;
        }

        TradeState::TradeAccepted
    }

    fn _store<E: Clone>(&self, event_map: &RwLock<HashMap<Uuid, E>>, id: Uuid, event: &E) {
        event_map.write().unwrap().insert(id, event.clone());
    }

    fn _do_only_in_state<F>(&self, uid: &Uuid, state: TradeState, f: F) -> Result<(), Error>
    where
        F: Fn(),
    {
        if self.current_state(uid) == state {
            f();
            Ok(())
        } else {
            Err(Error::IncorrectState)
        }
    }

    pub fn store_offer_created(&self, event: OfferCreated) -> Result<(), Error> {
        let uid = event.uid.clone();
        self._do_only_in_state(&uid, TradeState::NonExistent, || {
            self._store(&self.offer_created_events, uid, &event)
        })
    }

    pub fn store_trade_created(&self, event: TradeCreated) -> Result<(), Error> {
        let uid = event.uid.clone();
        self._do_only_in_state(&uid, TradeState::OfferCreated, || {
            self._store(&self.trade_created_events, uid, &event)
        })
    }

    pub fn store_trade_accepted(&self, event: TradeAccepted) -> Result<(), Error> {
        let uid = event.uid.clone();
        self._do_only_in_state(&uid, TradeState::TradeCreated, || {
            self._store(&self.trade_accepted_events, uid, &event);
        })
    }

    fn _get<E: Clone>(&self, event_map: &RwLock<HashMap<Uuid, E>>, id: &Uuid) -> Option<E> {
        event_map.read().unwrap().get(id).map(|x| x.clone())
    }

    pub fn get_offer_created(&self, id: &Uuid) -> Option<OfferCreated> {
        self._get(&self.offer_created_events, id)
    }

    pub fn get_trade_created(&self, id: &Uuid) -> Option<TradeCreated> {
        self._get(&self.trade_created_events, id)
    }

    pub fn get_trade_accept(&self, id: &Uuid) -> Option<TradeAccepted> {
        self._get(&self.trade_accepted_events, id)
    }
}
