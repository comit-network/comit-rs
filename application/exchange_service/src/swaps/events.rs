use common_types::{
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum, Ledger},
    secret::SecretHash,
    TradingSymbol,
};
use event_store::Event;
use secp256k1_support::KeyPair;
use std::marker::PhantomData;
use swaps::common::TradeId;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OfferCreated<B: Ledger, S: Ledger> {
    pub uid: TradeId,
    pub symbol: TradingSymbol,
    pub rate: f64,
    pub buy_amount: B::Quantity,
    pub sell_amount: S::Quantity,
    // TODO: treasury_expiry_timestamp
}
#[derive(Clone, Debug)]
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContractDeployed<T> {
    pub uid: TradeId,
    pub transaction_id: String,
    phantom: PhantomData<T>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContractRedeemed<T> {
    pub uid: TradeId,
    pub transaction_id: String,
    phantom: PhantomData<T>,
}

impl<B: Ledger, S: Ledger> Event for OfferCreated<B, S> {
    type Prev = ();
}

impl<B: Ledger> ContractDeployed<B> {
    pub fn new(uid: TradeId, transaction_id: String) -> ContractDeployed<B> {
        ContractDeployed {
            uid,
            transaction_id,
            phantom: PhantomData,
        }
    }
}

impl<B: Ledger> ContractRedeemed<B> {
    pub fn new(uid: TradeId, transaction_id: String) -> ContractRedeemed<B> {
        ContractRedeemed {
            uid,
            transaction_id,
            phantom: PhantomData,
        }
    }
}

impl<B: Ledger, S: Ledger> OfferCreated<B, S> {
    pub fn new(
        rate: f64,
        buy_amount: B::Quantity,
        sell_amount: S::Quantity,
        symbol: TradingSymbol,
    ) -> Self {
        OfferCreated {
            uid: TradeId::new(),
            symbol,
            rate,
            buy_amount,
            sell_amount,
        }
    }
}

impl<B: Ledger, S: Ledger> Event for OrderTaken<B, S> {
    type Prev = OfferCreated<B, S>;
}

impl Event for TradeFunded<Ethereum> {
    type Prev = OrderTaken<Bitcoin, Ethereum>;
}

impl Event for TradeFunded<Bitcoin> {
    type Prev = OrderTaken<Ethereum, Bitcoin>;
}

impl Event for ContractDeployed<Bitcoin> {
    type Prev = TradeFunded<Ethereum>;
}

impl Event for ContractDeployed<Ethereum> {
    type Prev = TradeFunded<Bitcoin>;
}

impl Event for ContractRedeemed<Ethereum> {
    type Prev = TradeFunded<Ethereum>;
}

impl Event for ContractRedeemed<Bitcoin> {
    type Prev = TradeFunded<Bitcoin>;
}
