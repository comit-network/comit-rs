use common_types::ledger::{bitcoin::Bitcoin, ethereum::Ethereum};
use ethereum_support::*;
use event_store::Event;
pub use swaps::eth_btc::common::{OfferCreated as OfferState, OfferCreated};
use swaps::{
    eth_btc::common::{OrderTaken, TradeFunded},
    TradeId,
};
use treasury_api_client::RateResponseBody;
use uuid::Uuid;

impl From<RateResponseBody> for OfferCreated<Ethereum, Bitcoin> {
    fn from(r: RateResponseBody) -> Self {
        OfferCreated {
            uid: TradeId(Uuid::new_v4()),
            symbol: r.symbol,
            rate: r.rate,
            buy_amount: r.buy_amount,   // ETH
            sell_amount: r.sell_amount, // BTC
        }
    }
}

impl Event for OrderTaken<Ethereum, Bitcoin> {
    type Prev = OfferCreated<Ethereum, Bitcoin>;
}

impl Event for TradeFunded<Bitcoin> {
    type Prev = OrderTaken<Ethereum, Bitcoin>;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContractDeployed {
    pub uid: TradeId,
    pub transaction_id: H256,
}

impl Event for ContractDeployed {
    type Prev = TradeFunded<Bitcoin>;
}
