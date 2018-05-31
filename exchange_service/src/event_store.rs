pub use self::OfferCreated as OfferState;
use bitcoin_rpc;
use eth_htlc;
use std::collections::HashMap;
use std::fmt;
use std::sync::RwLock;
use std::time::Duration;
use std::time::SystemTime;
use treasury_api_client::Symbol;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OfferCreated {
    pub uid: Uuid,
    pub symbol: Symbol,
    pub amount: u32,
    pub rate: f32,
    // TODO: treasury_expiry_timestamp
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrderTaken {
    uid: Uuid,

    contract_secret_lock: eth_htlc::SecretHash,
    client_contract_time_lock: bitcoin_rpc::BlockHeight,
    exchange_contract_time_lock: SystemTime,

    client_refund_address: bitcoin_rpc::Address,
    client_success_address: eth_htlc::Address,

    exchange_refund_address: eth_htlc::Address,
    exchange_success_address: bitcoin_rpc::Address,
}

impl OrderTaken {
    pub fn new(
        uid: Uuid,

        contract_secret_lock: eth_htlc::SecretHash,
        client_contract_time_lock: bitcoin_rpc::BlockHeight,

        client_refund_address: bitcoin_rpc::Address,
        client_success_address: eth_htlc::Address,
        exchange_refund_address: eth_htlc::Address,
        exchange_success_address: bitcoin_rpc::Address,
    ) -> Self {
        let twelve_hours = Duration::new(60 * 60 * 12, 0);

        OrderTaken {
            uid,

            contract_secret_lock,
            client_contract_time_lock,
            exchange_contract_time_lock: SystemTime::now() + twelve_hours,

            client_refund_address,
            client_success_address,
            exchange_refund_address,
            exchange_success_address,
        }
    }

    pub fn exchange_success_address(&self) -> bitcoin_rpc::Address {
        self.exchange_success_address.clone()
    }

    pub fn exchange_refund_address(&self) -> eth_htlc::Address {
        self.exchange_refund_address
    }

    pub fn exchange_contract_time_lock(&self) -> SystemTime {
        self.exchange_contract_time_lock
    }

    pub fn client_refund_address(&self) -> bitcoin_rpc::Address {
        self.client_refund_address.clone()
    }

    pub fn contract_secret_lock(&self) -> eth_htlc::SecretHash {
        self.contract_secret_lock
    }
}

#[derive(Debug, PartialEq)]
enum TradeState {
    // Offer has been requested and answered
    OfferCreated,
    // Order has been requested and all details provided to move forward. Now waiting for address to be funded.
    OrderTaken,
}

pub struct EventStore {
    states: RwLock<HashMap<Uuid, TradeState>>,
    offers: RwLock<HashMap<Uuid, OfferCreated>>,
    order_taken: RwLock<HashMap<Uuid, OrderTaken>>,
}

#[derive(Debug)]
pub enum Error {
    UnexpectedState,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Error::UnexpectedState => write!(
                f,
                "UnexpectedState: Known state for the given uid does not match the query"
            ),
        }
    }
}

impl EventStore {
    pub fn new() -> EventStore {
        EventStore {
            states: RwLock::new(HashMap::new()),
            offers: RwLock::new(HashMap::new()),
            order_taken: RwLock::new(HashMap::new()),
        }
    }

    /* To uncomment when needed
    pub fn get_offer(&self, id: &Uuid) -> Option<OfferState> {
        let offers = self.offers.read().unwrap();
        offers.get(id).map(|offer| offer.clone())
    }
    */

    pub fn store_offer(&self, event: OfferCreated) -> Result<(), Error> {
        let uid = event.uid.clone();
        let mut states = self.states.write().unwrap();

        match states.get(&uid) {
            Some(_) => return Err(Error::UnexpectedState),
            None => states.insert(uid, TradeState::OfferCreated),
        };

        {
            let mut offers = self.offers.write().unwrap();
            offers.insert(uid, event.clone());
        }
        Ok(())
    }

    pub fn store_order_taken(&self, event: OrderTaken) -> Result<(), Error> {
        let uid = event.uid.clone();
        let mut states = self.states.write().unwrap();

        match states.get_mut(&uid) {
            Some(ref mut state) if **state == TradeState::OfferCreated => {
                **state = TradeState::OrderTaken
            }
            _ => return Err(Error::UnexpectedState),
        }

        {
            let mut order_taken = self.order_taken.write().unwrap();
            order_taken.insert(uid, event.clone());
        }
        Ok(())
    }

    /*pub fn get_trade(&self, id: &Uuid) -> Option<TradeState> {
        let trades = self.trades.read().unwrap();
        trades.get(id).map(|trade| trade.clone())
    }*/
}
