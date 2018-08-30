//TODO: create and use bitcoin_support blockheight
use bitcoin_rpc_client::{BlockHeight, TransactionId};
use bitcoin_support::{Address, BitcoinQuantity};
use ledger::Ledger;

#[derive(Clone, Debug)]
pub struct Bitcoin {}

#[derive(Clone, Deserialize, Serialize)]
pub struct HtlcId {
    pub transaction_id: TransactionId,
    pub vout: u32,
}

impl Ledger for Bitcoin {
    type Quantity = BitcoinQuantity;
    type Address = Address;
    type LockDuration = BlockHeight;
    type HtlcId = HtlcId;

    fn symbol() -> String {
        String::from("BTC")
    }
}
