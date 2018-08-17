use bitcoin_fee_service;
use bitcoin_rpc;
use common_types::{ledger::Ledger, secret::SecretHash, TradingSymbol};
use ethereum_service;
use event_store::{self, Event};
use reqwest;
use secp256k1_support::KeyPair;
use swaps::TradeId;

#[derive(Debug)]
pub enum Error {
    EventStore(event_store::Error),
    TreasuryService(reqwest::Error),
    FeeService(bitcoin_fee_service::Error),
    EthereumService(ethereum_service::Error),
    BitcoinRpc(bitcoin_rpc::RpcError),
    BitcoinNode(reqwest::Error),
    Unlocking(String),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OfferCreated<B: Ledger, S: Ledger> {
    pub uid: TradeId,
    pub symbol: TradingSymbol,
    pub rate: f64,
    pub buy_amount: B::Quantity,
    pub sell_amount: S::Quantity,
    // TODO: treasury_expiry_timestamp
}

impl<B: Ledger, S: Ledger> Event for OfferCreated<B, S> {
    type Prev = ();
}

#[derive(Clone)]
pub struct OrderTaken<B: Ledger, S: Ledger> {
    pub uid: TradeId,

    pub contract_secret_lock: SecretHash,
    pub client_contract_time_lock: S::Time,
    pub exchange_contract_time_lock: B::Time,

    pub client_refund_address: S::Address,
    pub client_success_address: B::Address,

    pub exchange_refund_address: B::Address,
    pub exchange_success_address: S::Address,
    pub exchange_success_keypair: KeyPair,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TradeFunded<S: Ledger> {
    pub uid: TradeId,
    pub htlc_identifier: S::HtlcId,
}
