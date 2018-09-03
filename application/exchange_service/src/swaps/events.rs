use common_types::{ledger::Ledger, secret::SecretHash};
use event_store::Event;
use secp256k1_support::KeyPair;
use std::marker::PhantomData;
use swaps::common::TradeId;

#[derive(Clone, Debug)]
pub struct OrderTaken<Buy: Ledger, Sell: Ledger> {
    pub uid: TradeId,

    pub contract_secret_lock: SecretHash,
    pub client_contract_time_lock: Sell::LockDuration,
    pub exchange_contract_time_lock: Buy::LockDuration,

    pub client_refund_address: Sell::Address,
    pub client_success_address: Buy::Address,

    pub exchange_refund_address: Buy::Address,
    pub exchange_success_address: Sell::Address,
    pub exchange_success_keypair: KeyPair,

    pub buy_amount: Buy::Quantity,
    pub sell_amount: Sell::Quantity,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TradeFunded<Buy: Ledger, Sell: Ledger> {
    pub uid: TradeId,
    pub htlc_identifier: Sell::HtlcId,
    phantom: PhantomData<Buy>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContractDeployed<Buy: Ledger, Sell: Ledger> {
    pub uid: TradeId,
    pub transaction_id: String,
    phantom: PhantomData<Buy>,
    phantom2: PhantomData<Sell>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContractRedeemed<Buy: Ledger, Sell: Ledger> {
    pub uid: TradeId,
    pub transaction_id: String,
    phantom: PhantomData<Buy>,
    phantom2: PhantomData<Sell>,
}

impl<Buy: Ledger, Sell: Ledger> ContractDeployed<Buy, Sell> {
    pub fn new(uid: TradeId, transaction_id: String) -> ContractDeployed<Buy, Sell> {
        ContractDeployed {
            uid,
            transaction_id,
            phantom: PhantomData,
            phantom2: PhantomData,
        }
    }
}

impl<Buy: Ledger, Sell: Ledger> TradeFunded<Buy, Sell> {
    pub fn new(uid: TradeId, htlc_identifier: Sell::HtlcId) -> TradeFunded<Buy, Sell> {
        TradeFunded {
            uid,
            htlc_identifier,
            phantom: PhantomData,
        }
    }
}

impl<Buy: Ledger, Sell: Ledger> ContractRedeemed<Buy, Sell> {
    pub fn new(uid: TradeId, transaction_id: String) -> ContractRedeemed<Buy, Sell> {
        ContractRedeemed {
            uid,
            transaction_id,
            phantom: PhantomData,
            phantom2: PhantomData,
        }
    }
}

impl<Buy: Ledger, Sell: Ledger> Event for OrderTaken<Buy, Sell> {
    type Prev = ();
}

impl<Buy: Ledger, Sell: Ledger> Event for TradeFunded<Buy, Sell> {
    type Prev = OrderTaken<Buy, Sell>;
}

impl<Buy: Ledger, Sell: Ledger> Event for ContractDeployed<Buy, Sell> {
    type Prev = TradeFunded<Buy, Sell>;
}

impl<Buy: Ledger, Sell: Ledger> Event for ContractRedeemed<Buy, Sell> {
    type Prev = TradeFunded<Buy, Sell>;
}
