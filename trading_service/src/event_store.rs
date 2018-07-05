use bitcoin_htlc::Htlc;
use bitcoin_rpc;
use bitcoin_rpc::BlockHeight;
use common_types::{BitcoinQuantity, EthereumQuantity};
use exchange_api_client::OfferResponseBody;
use secret::Secret;
use std::collections::HashMap;
use std::fmt;
use std::sync::RwLock;
use symbol::Symbol;
use uuid::Uuid;
use web3::types::Address as EthAddress;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TradeId(Uuid);

impl TradeId {
    pub fn from_uuid(uuid: Uuid) -> Self {
        TradeId(uuid)
    }
}

impl fmt::Display for TradeId {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.0.fmt(f)
    }
}

// State after exchange has made an offer
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OfferCreated {
    pub uid: TradeId,
    pub symbol: Symbol,
    pub rate: f64,
    pub btc_amount: BitcoinQuantity,
    pub eth_amount: EthereumQuantity,
}

impl From<OfferResponseBody> for OfferCreated {
    fn from(offer: OfferResponseBody) -> Self {
        OfferCreated {
            uid: offer.uid,
            symbol: offer.symbol,
            rate: offer.rate,
            btc_amount: offer.btc_amount,
            eth_amount: offer.eth_amount,
        }
    }
}

// State after client accepts trade offer
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrderCreated {
    pub uid: TradeId,
    pub client_success_address: EthAddress,
    pub client_refund_address: bitcoin_rpc::RpcAddress,
    pub secret: Secret,
    pub long_relative_timelock: BlockHeight,
}

#[derive(Clone, Debug)]
pub struct OrderTaken {
    pub uid: TradeId,
    pub exchange_refund_address: EthAddress,
    // This is embedded in the HTLC but we keep it here as well for completeness
    pub exchange_success_address: bitcoin_rpc::RpcAddress,
    pub exchange_contract_time_lock: u64,
    pub htlc: Htlc,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContractDeployed {
    pub uid: TradeId,
    pub address: EthAddress,
}

pub struct EventStore {
    offer_created: RwLock<HashMap<TradeId, OfferCreated>>,
    order_created: RwLock<HashMap<TradeId, OrderCreated>>,
    order_taken: RwLock<HashMap<TradeId, OrderTaken>>,
    contract_deployed: RwLock<HashMap<TradeId, ContractDeployed>>,
}

#[derive(PartialEq)]
enum TradeState {
    NonExistent,
    OfferCreated,
    OrderCreated,
    OrderTaken,
    ContractDeployed,
}

#[derive(Debug)]
pub enum Error {
    IncorrectState,
}

impl EventStore {
    pub fn new() -> EventStore {
        EventStore {
            offer_created: RwLock::new(HashMap::new()),
            order_created: RwLock::new(HashMap::new()),
            order_taken: RwLock::new(HashMap::new()),
            contract_deployed: RwLock::new(HashMap::new()),
        }
    }

    fn current_state(&self, id: &TradeId) -> TradeState {
        if self._get(&self.offer_created, id).is_none() {
            return TradeState::NonExistent;
        }

        if self._get(&self.order_created, id).is_none() {
            return TradeState::OfferCreated;
        }

        if self._get(&self.order_taken, id).is_none() {
            return TradeState::OrderCreated;
        }

        if self._get(&self.contract_deployed, id).is_none() {
            return TradeState::OrderTaken;
        }

        TradeState::ContractDeployed
    }

    fn _store<E: Clone>(
        &self,
        event_map: &RwLock<HashMap<TradeId, E>>,
        required_state: TradeState,
        id: TradeId,
        event: &E,
    ) -> Result<(), Error> {
        if self.current_state(&id) == required_state {
            event_map.write().unwrap().insert(id, event.clone());
            Ok(())
        } else {
            Err(Error::IncorrectState)
        }
    }

    pub fn store_offer_created(&self, event: OfferCreated) -> Result<(), Error> {
        let uid = event.uid.clone();
        self._store(&self.offer_created, TradeState::NonExistent, uid, &event)
    }

    pub fn store_trade_created(&self, event: OrderCreated) -> Result<(), Error> {
        let uid = event.uid.clone();
        self._store(&self.order_created, TradeState::OfferCreated, uid, &event)
    }

    pub fn store_trade_accepted(&self, event: OrderTaken) -> Result<(), Error> {
        let uid = event.uid.clone();
        self._store(&self.order_taken, TradeState::OrderCreated, uid, &event)
    }

    pub fn store_contract_deployed(&self, event: ContractDeployed) -> Result<(), Error> {
        let uid = event.uid.clone();
        self._store(&self.contract_deployed, TradeState::OrderTaken, uid, &event)
    }

    fn _get<E: Clone>(&self, event_map: &RwLock<HashMap<TradeId, E>>, id: &TradeId) -> Option<E> {
        event_map.read().unwrap().get(id).map(Clone::clone)
    }

    pub fn get_offer_created(&self, id: &TradeId) -> Result<OfferCreated, Error> {
        self._get(&self.offer_created, id)
            .map_or(Err(Error::IncorrectState), |event| Ok(event.clone()))
    }

    pub fn get_order_created(&self, id: &TradeId) -> Result<OrderCreated, Error> {
        self._get(&self.order_created, id)
            .map_or(Err(Error::IncorrectState), |event| Ok(event.clone()))
    }

    pub fn get_order_taken(&self, id: &TradeId) -> Result<OrderTaken, Error> {
        self._get(&self.order_taken, id)
            .map_or(Err(Error::IncorrectState), |event| Ok(event.clone()))
    }

    pub fn get_contract_deployed(&self, id: &TradeId) -> Result<ContractDeployed, Error> {
        self._get(&self.contract_deployed, id)
            .map_or(Err(Error::IncorrectState), |event| Ok(event.clone()))
    }
}
