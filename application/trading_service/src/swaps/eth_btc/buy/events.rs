use bitcoin_htlc::Htlc;
use bitcoin_rpc::{self, BlockHeight};
use bitcoin_support::BitcoinQuantity;
use common_types::{
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum},
    TradingSymbol,
};

use common_types::ledger::Ledger;
use ethereum_support::EthereumQuantity;
use event_store::Event;
use exchange_api_client::OfferResponseBody;
use secret::Secret;
use swaps::TradeId;

use std::str::FromStr;

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

impl From<OfferResponseBody> for OfferCreated<Ethereum, Bitcoin> {
    fn from(offer: OfferResponseBody) -> Self {
        OfferCreated {
            uid: offer.uid,
            symbol: offer.symbol,
            rate: offer.rate,
            buy_amount: EthereumQuantity::from_str(offer.buy_amount.as_str()).unwrap(),
            sell_amount: BitcoinQuantity::from_str(offer.sell_amount.as_str()).unwrap(),
        }
    }
}

impl Event for OfferCreated<Ethereum, Bitcoin> {
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
    pub long_relative_timelock: BlockHeight,
}

impl Event for OrderCreated<Ethereum, Bitcoin> {
    type Prev = OfferCreated<Ethereum, Bitcoin>;
}

#[derive(Clone, Debug)]
pub struct OrderTaken<B>
where
    B: Ledger,
{
    pub uid: TradeId,
    pub exchange_refund_address: B::Address,
    // This is embedded in the HTLC but we keep it here as well for completeness
    pub exchange_success_address: bitcoin_rpc::Address, // todo change this to bitcoin_support
    pub exchange_contract_time_lock: u64,
    pub htlc: Htlc,
}

impl Event for OrderTaken<Ethereum> {
    type Prev = OrderCreated<Ethereum, Bitcoin>;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContractDeployed<B>
where
    B: Ledger,
{
    pub uid: TradeId,
    pub address: B::Address,
}

impl Event for ContractDeployed<Ethereum> {
    type Prev = OrderTaken<Ethereum>;
}
