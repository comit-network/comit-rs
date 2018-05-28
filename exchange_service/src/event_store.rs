pub use self::OfferEvent as OfferState;
use bitcoin_rpc;
pub use routes::eth_btc::OfferRequestResponse as OfferEvent;
use std::collections::HashMap;
use std::sync::RwLock;
use types::{BtcBlockHeight, EthAddress, EthTimestamp, SecretHash};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TradeEvent {
    pub uid: Uuid,
    pub secret_hash: SecretHash,
    pub client_refund_address: bitcoin_rpc::Address,
    pub long_relative_time_lock: BtcBlockHeight,
    pub short_relative_time_lock: EthTimestamp,
    pub client_success_address: EthAddress,
    pub exchange_refund_address: EthAddress,
}

pub enum State {
    // Offer has been requested and answered
    Offer,
    // Trade/Order has been requested and all details provided to move forward. Now waiting for address to be funded
    Trade,
}

pub struct EventStore {
    states: RwLock<HashMap<Uuid, RwLock<State>>>,
    offers: RwLock<HashMap<Uuid, OfferEvent>>,
    trades: RwLock<HashMap<Uuid, TradeEvent>>,
}

#[derive(Debug)]
pub enum Error {
    IncorrectState,
}

impl EventStore {
    pub fn new() -> EventStore {
        EventStore {
            states: RwLock::new(HashMap::new()),
            offers: RwLock::new(HashMap::new()),
            trades: RwLock::new(HashMap::new()),
        }
    }

    pub fn store_offer(&self, event: OfferEvent) -> Result<(), Error> {
        let uid = event.uid.clone();
        let mut states = self.states.write().unwrap();

        match states.get(&uid) {
            Some(_state) => return Err(Error::IncorrectState),
            None => (),
        }
        states.insert(uid, RwLock::new(State::Offer));

        {
            let mut offers = self.offers.write().unwrap();
            offers.insert(uid, event.clone());
        }
        Ok(())
    }

    /* To uncomment when needed
    pub fn get_offer(&self, id: &Uuid) -> Option<OfferState> {
        let offers = self.offers.read().unwrap();
        offers.get(id).map(|offer| offer.clone())
    }
    */

    pub fn store_trade(&self, event: TradeEvent) -> Result<(), Error> {
        let uid = event.uid.clone();
        let mut states = self.states.write().unwrap();

        match states.get_mut(&uid) {
            None => return Err(Error::IncorrectState),
            Some(state) => {
                let mut state = state.write().unwrap();
                match *state {
                    State::Offer => *state = State::Trade,
                    _ => return Err(Error::IncorrectState),
                }
            }
        }
        {
            let mut trades = self.trades.write().unwrap();
            trades.insert(uid, event.clone());
        }
        Ok(())
    }

    /*pub fn get_trade(&self, id: &Uuid) -> Option<TradeState> {
        let trades = self.trades.read().unwrap();
        trades.get(id).map(|trade| trade.clone())
    }*/
}
