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

impl OfferCreated<Bitcoin, Ethereum> {
    fn new(r: RateResponseBody, buy_amount: BitcoinQuantity) -> Self {
        OfferCreated {
            uid: TradeId(Uuid::new_v4()),
            symbol: TradingSymbol::ETH_BTC,
            rate: r.rate,
            buy_amount,
            // TODO: Fail nicely if rate == 0
            sell_amount: EthereumQuantity::from_eth(buy_amount.bitcoin() / r.rate), // ETH
        }
    }
}

impl Event for OrderTaken<Bitcoin, Ethereum> {
    type Prev = OfferCreated<Bitcoin, Ethereum>;
}

impl Event for TradeFunded<Ethereum> {
    type Prev = OrderTaken<Bitcoin, Ethereum>;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContractDeployed {
    pub uid: TradeId,
    pub transaction_id: H256,
}

impl Event for ContractDeployed {
    type Prev = TradeFunded<Ethereum>;
}
