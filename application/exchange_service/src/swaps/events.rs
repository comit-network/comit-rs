use common_types::{
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum},
    TradingSymbol,
};
use event_store::Event;
use secp256k1_support::KeyPair;
use swaps::common::TradeId;
// todo check why OfferCreated as OfferState
use common_types::{ledger::Ledger, secret::SecretHash};
use std::marker::PhantomData;

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

//impl OfferCreated<Ethereum, Bitcoin> {
//    pub fn new(r: RateResponseBody, buy_amount: EthereumQuantity) -> Self {
//        OfferCreated {
//            uid: TradeId(Uuid::new_v4()),
//            symbol: TradingSymbol::ETH_BTC,
//            rate: r.rate,
//            buy_amount,
//            sell_amount: BitcoinQuantity::from_bitcoin(r.rate * buy_amount.ethereum()),
//        }
//    }
//}
//
//impl OfferCreated<Bitcoin, Ethereum> {
//    fn new(r: RateResponseBody, buy_amount: BitcoinQuantity) -> Self {
//        OfferCreated {
//            uid: TradeId(Uuid::new_v4()),
//            symbol: TradingSymbol::ETH_BTC,
//            rate: r.rate,
//            buy_amount,
//            // TODO: Fail nicely if rate == 0
//            sell_amount: EthereumQuantity::from_eth(buy_amount.bitcoin() / r.rate), // ETH
//        }
//    }
//}

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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContractDeployed<T> {
    pub uid: TradeId,
    pub transaction_id: String,
    phantom: PhantomData<T>,
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

//series of events is as follows:
// OfferCreated buy ETH for BTC -> OrderTaken ETH for BTC-> TradeFunded BTC from trader -> ContractDeployed ETH from exchange
