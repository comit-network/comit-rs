pub use self::OfferCreated as OfferState;
use bitcoin_rpc;
use bitcoin_support::{self, BitcoinQuantity};
use common_types::secret::SecretHash;
use ethereum_support::{self, *};
use event_store::Event;
use events_common::TradeId;
use secp256k1_support::KeyPair;
use std::time::SystemTime;
use treasury_api_client::{RateResponseBody, Symbol};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OfferCreated {
    pub uid: TradeId,
    pub symbol: Symbol,
    pub rate: f64,
    pub eth_amount: EthereumQuantity,
    pub btc_amount: BitcoinQuantity,
    // TODO: treasury_expiry_timestamp
}

impl Event for OfferCreated {
    type Prev = ();
}

impl From<RateResponseBody> for OfferCreated {
    fn from(r: RateResponseBody) -> Self {
        OfferCreated {
            uid: TradeId(Uuid::new_v4()),
            symbol: Symbol(r.symbol),
            rate: r.rate,
            eth_amount: r.buy_amount,
            btc_amount: r.sell_amount,
        }
    }
}

#[derive(Clone)]
pub struct OrderTaken {
    pub uid: TradeId,

    pub contract_secret_lock: SecretHash,
    pub client_contract_time_lock: bitcoin_rpc::BlockHeight,
    pub exchange_contract_time_lock: SystemTime,

    pub client_refund_address: bitcoin_support::Address,
    pub client_success_address: ethereum_support::Address,

    pub exchange_refund_address: ethereum_support::Address,
    pub exchange_success_address: bitcoin_support::Address,
    pub exchange_success_keypair: KeyPair,
}

impl Event for OrderTaken {
    type Prev = OfferCreated;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TradeFunded {
    pub uid: TradeId,
    pub transaction_id: bitcoin_rpc::TransactionId,
    pub vout: u32,
}

impl Event for TradeFunded {
    type Prev = OrderTaken;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContractDeployed {
    pub uid: TradeId,
    pub transaction_id: H256,
}

impl Event for ContractDeployed {
    type Prev = TradeFunded;
}
