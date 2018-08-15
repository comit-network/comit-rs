use bitcoin_support::*;
use common_types::{
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum},
    TradingSymbol,
};
use ethereum_support::*;
use event_store::Event;
pub use swaps::eth_btc::common::{OfferCreated as OfferState, OfferCreated};
use swaps::{
    eth_btc::common::{OrderTaken, TradeFunded},
    TradeId,
};
use treasury_api_client::RateResponseBody;
use uuid::Uuid;

impl OfferCreated<Ethereum, Bitcoin> {
    pub fn new(r: RateResponseBody, buy_amount: EthereumQuantity) -> Self {
        OfferCreated {
            uid: TradeId(Uuid::new_v4()),
            symbol: TradingSymbol::ETH_BTC,
            rate: r.rate,
            buy_amount,
            sell_amount: BitcoinQuantity::from_bitcoin(r.rate * buy_amount.ethereum()),
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
