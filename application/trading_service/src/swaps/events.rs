use common_types::{ledger::Ledger, TradingSymbol};
use event_store::Event;
use exchange_api_client::OfferResponseBody;
use secret::Secret;
use std::marker::PhantomData;
use swaps::TradeId;

// State after exchange has made an offer
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OfferCreated<B, S>
where
    B: Ledger,
    S: Ledger,
{
    pub uid: TradeId,
    pub symbol: TradingSymbol,
    pub rate: f64,
    pub buy_amount: B::Quantity,
    pub sell_amount: S::Quantity,
}

impl<B: Ledger, S: Ledger> From<OfferResponseBody<B, S>> for OfferCreated<B, S> {
    fn from(offer: OfferResponseBody<B, S>) -> Self {
        OfferCreated {
            uid: offer.uid,
            symbol: offer.symbol,
            rate: offer.rate,
            buy_amount: offer.buy_amount,
            sell_amount: offer.sell_amount,
        }
    }
}

impl<B: Ledger, S: Ledger> Event for OfferCreated<B, S> {
    type Prev = ();
}

// State after client accepts trade offer
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrderCreated<B, S>
where
    B: Ledger,
    S: Ledger,
{
    pub uid: TradeId,
    pub client_success_address: B::Address,
    pub client_refund_address: S::Address,
    pub secret: Secret,
    pub long_relative_timelock: S::Time,
}

impl<B: Ledger, S: Ledger> Event for OrderCreated<B, S> {
    type Prev = OfferCreated<B, S>;
}

#[derive(Clone, Debug)]
pub struct OrderTaken<B, S>
where
    B: Ledger,
    S: Ledger,
{
    pub uid: TradeId,
    pub exchange_refund_address: B::Address,
    pub exchange_success_address: S::Address,
    pub exchange_contract_time_lock: B::Time,
}

impl<B: Ledger, S: Ledger> Event for OrderTaken<B, S> {
    type Prev = OrderCreated<B, S>;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContractDeployed<B, S>
where
    B: Ledger,
    S: Ledger,
{
    pub uid: TradeId,
    pub address: B::Address,
    phantom: PhantomData<S>,
}

impl<B: Ledger, S: Ledger> ContractDeployed<B, S> {
    pub fn new(uid: TradeId, address: B::Address) -> ContractDeployed<B, S> {
        ContractDeployed {
            uid,
            address,
            phantom: PhantomData,
        }
    }
}

impl<B: Ledger, S: Ledger> Event for ContractDeployed<B, S> {
    type Prev = OrderTaken<B, S>;
}
