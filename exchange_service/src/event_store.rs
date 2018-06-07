pub use self::OfferCreated as OfferState;
use bitcoin_rpc;
use bitcoin_wallet;
use common_types::secret::SecretHash;
use common_types::{BitcoinQuantity, EthQuantity};
use std::collections::HashMap;
use std::fmt;
use std::sync::RwLock;
use std::time::Duration;
use std::time::SystemTime;
use treasury_api_client::Symbol;
use uuid::Uuid;
use web3::types::Address as EthAddress;
use web3::types::H256;

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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OfferCreated {
    uid: TradeId,
    symbol: Symbol,
    rate: f32,
    eth_amount: EthQuantity,
    btc_amount: BitcoinQuantity,
    // TODO: treasury_expiry_timestamp
}

impl OfferCreated {
    pub fn new(
        symbol: Symbol,
        rate: f32,
        eth_amount: EthQuantity,
        btc_amount: BitcoinQuantity,
    ) -> Self {
        OfferCreated {
            uid: TradeId(Uuid::new_v4()),
            symbol,
            eth_amount,
            btc_amount,
            rate,
        }
    }

    pub fn btc_amount(&self) -> BitcoinQuantity {
        self.btc_amount
    }
}

#[derive(Clone)]
pub struct OrderTaken {
    uid: TradeId,

    contract_secret_lock: SecretHash,
    client_contract_time_lock: bitcoin_rpc::BlockHeight,
    exchange_contract_time_lock: SystemTime,

    client_refund_address: bitcoin_rpc::Address,
    client_success_address: EthAddress,

    exchange_refund_address: EthAddress,
    exchange_success_address: bitcoin_rpc::Address,
    exchange_success_private_key: bitcoin_wallet::PrivateKey,
}

impl OrderTaken {
    pub fn new(
        uid: TradeId,

        contract_secret_lock: SecretHash,
        client_contract_time_lock: bitcoin_rpc::BlockHeight,

        client_refund_address: bitcoin_rpc::Address,
        client_success_address: EthAddress,
        exchange_refund_address: EthAddress,
        exchange_success_address: bitcoin_rpc::Address,
        exchange_success_private_key: bitcoin_wallet::PrivateKey,
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
            exchange_success_private_key,
        }
    }

    pub fn exchange_success_address(&self) -> bitcoin_rpc::Address {
        self.exchange_success_address.clone()
    }

    pub fn exchange_refund_address(&self) -> EthAddress {
        self.exchange_refund_address
    }

    pub fn exchange_contract_time_lock(&self) -> SystemTime {
        self.exchange_contract_time_lock
    }

    pub fn client_refund_address(&self) -> bitcoin_rpc::Address {
        self.client_refund_address.clone()
    }

    pub fn client_success_address(&self) -> EthAddress {
        self.client_success_address.clone()
    }

    pub fn contract_secret_lock(&self) -> &SecretHash {
        &self.contract_secret_lock
    }

    pub fn client_contract_time_lock(&self) -> &bitcoin_rpc::BlockHeight {
        &self.client_contract_time_lock
    }

    pub fn exchange_success_private_key(&self) -> &bitcoin_wallet::PrivateKey {
        &self.exchange_success_private_key
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContractDeployed {
    uid: TradeId,
    transaction_id: H256,
}

impl ContractDeployed {
    pub fn new(uid: TradeId, transaction_id: H256) -> Self {
        ContractDeployed {
            uid,
            transaction_id,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TradeFunded {
    uid: TradeId,
    transaction_id: bitcoin_rpc::TransactionId,
}

impl TradeFunded {
    pub fn new(uid: TradeId, transaction_id: bitcoin_rpc::TransactionId) -> Self {
        TradeFunded {
            uid,
            transaction_id,
        }
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
    states: RwLock<HashMap<TradeId, TradeState>>,
    offers: RwLock<HashMap<TradeId, OfferCreated>>,
    order_taken: RwLock<HashMap<TradeId, OrderTaken>>,
    contract_deployed: RwLock<HashMap<TradeId, ContractDeployed>>,
    trade_funded: RwLock<HashMap<TradeId, TradeFunded>>,
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
            contract_deployed: RwLock::new(HashMap::new()),
            trade_funded: RwLock::new(HashMap::new()),
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

    pub fn store_contract_deployed(&self, event: ContractDeployed) -> Result<(), Error> {
        let uid = event.uid.clone();

        {
            let mut events = self.contract_deployed.write().unwrap();

            if events.get(&uid).is_some() {
                return Err(Error::UnexpectedState);
            }

            events.insert(uid, event.clone());
        }

        Ok(())
    }

    pub fn store_trade_funded(&self, event: TradeFunded) -> Result<(), Error> {
        let uid = event.uid.clone();

        {
            let mut events = self.trade_funded.write().unwrap();

            if events.get(&uid).is_some() {
                return Err(Error::UnexpectedState);
            }

            events.insert(uid, event.clone());
        }

        Ok(())
    }

    pub fn get_offer_created_event(&self, uid: &TradeId) -> Option<OfferCreated> {
        let events = self.offers.read().unwrap();

        events.get(uid).map(|event| event.clone())
    }

    pub fn get_order_taken_event(&self, uid: &TradeId) -> Option<OrderTaken> {
        let events = self.order_taken.read().unwrap();

        events.get(uid).map(|event| event.clone())
    }

    pub fn get_contract_deployed_event(&self, uid: &TradeId) -> Option<ContractDeployed> {
        let events = self.contract_deployed.read().unwrap();

        events.get(uid).map(|event| event.clone())
    }

    pub fn get_trade_funded_event(&self, uid: &TradeId) -> Option<TradeFunded> {
        let events = self.trade_funded.read().unwrap();

        events.get(uid).map(|event| event.clone())
    }

    /*pub fn get_trade(&self, id: &Uuid) -> Option<TradeState> {
        let trades = self.trades.read().unwrap();
        trades.get(id).map(|trade| trade.clone())
    }*/
}
