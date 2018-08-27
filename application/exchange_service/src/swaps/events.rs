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
pub struct TradeFunded<B: Ledger, S: Ledger> {
    pub uid: TradeId,
    pub htlc_identifier: S::HtlcId,
    phantom: PhantomData<B>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContractDeployed<B: Ledger, S: Ledger> {
    pub uid: TradeId,
    pub transaction_id: String,
    phantom: PhantomData<B>,
    phantom2: PhantomData<S>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContractRedeemed<B: Ledger, S: Ledger> {
    pub uid: TradeId,
    pub transaction_id: String,
    phantom: PhantomData<B>,
    phantom2: PhantomData<S>,
}

impl<B: Ledger, S: Ledger> Event for OfferCreated<B, S> {
    type Prev = ();
}

impl<B: Ledger, S: Ledger> ContractDeployed<B, S> {
    pub fn new(uid: TradeId, transaction_id: String) -> ContractDeployed<B, S> {
        ContractDeployed {
            uid,
            transaction_id,
            phantom: PhantomData,
            phantom2: PhantomData,
        }
    }
}

impl<B: Ledger, S: Ledger> TradeFunded<B, S> {
    pub fn new(uid: TradeId, htlc_identifier: S::HtlcId) -> TradeFunded<B, S> {
        TradeFunded {
            uid,
            htlc_identifier,
            phantom: PhantomData,
        }
    }
}

impl<B: Ledger, S: Ledger> ContractRedeemed<B, S> {
    pub fn new(uid: TradeId, transaction_id: String) -> ContractRedeemed<B, S> {
        ContractRedeemed {
            uid,
            transaction_id,
            phantom: PhantomData,
            phantom2: PhantomData,
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

impl<B: Ledger, S: Ledger> Event for TradeFunded<B, S> {
    type Prev = OrderTaken<B, S>;
}

impl<B: Ledger, S: Ledger> Event for ContractDeployed<B, S> {
    type Prev = TradeFunded<B, S>;
}

impl<B: Ledger, S: Ledger> Event for ContractRedeemed<B, S> {
    type Prev = TradeFunded<B, S>;
}
