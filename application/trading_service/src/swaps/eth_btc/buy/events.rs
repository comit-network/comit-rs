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
pub struct OfferCreated {
    pub uid: TradeId,
    pub symbol: TradingSymbol,
    pub rate: f64,
    pub btc_amount: BitcoinQuantity,
    pub eth_amount: EthereumQuantity,
}

impl From<OfferResponseBody> for OfferCreated {
    fn from(offer: OfferResponseBody) -> Self {
        OfferCreated {
            uid: offer.uid,
            symbol: offer.symbol,
            rate: offer.rate,
            btc_amount: offer.btc_amount,
            eth_amount: offer.eth_amount,
        }
    }
}

impl Event for OfferCreated {
    type Prev = ();
}

// State after client accepts trade offer
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrderCreated {
    pub uid: TradeId,
    pub client_success_address: ethereum_support::Address,
    pub client_refund_address: bitcoin_rpc::Address,
    pub secret: Secret,
    pub long_relative_timelock: BlockHeight,
}

impl Event for OrderCreated {
    type Prev = OfferCreated;
}

#[derive(Clone, Debug)]
pub struct OrderTaken {
    pub uid: TradeId,
    pub exchange_refund_address: ethereum_support::Address,
    // This is embedded in the HTLC but we keep it here as well for completeness
    pub exchange_success_address: bitcoin_rpc::Address,
    pub exchange_contract_time_lock: u64,
    pub htlc: Htlc,
}

impl Event for OrderTaken {
    type Prev = OrderCreated;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContractDeployed {
    pub uid: TradeId,
    pub address: ethereum_support::Address,
}

impl Event for ContractDeployed {
    type Prev = OrderTaken;
}
