use common_types::{ledger::Ledger, TradingSymbol};
use event_store::Event;
use swaps::TradeId;

//TODO looks common to buy/sell
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
