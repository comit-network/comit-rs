use bitcoin_rpc_client::TransactionId;
use bitcoin_support::{Address, BitcoinQuantity, Blocks, Network, PubkeyHash};
use ledger::Ledger;
use secp256k1_support::PublicKey;
use swap;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Bitcoin {}

#[derive(Clone, Deserialize, Serialize)]
pub struct HtlcId {
    pub transaction_id: TransactionId,
    pub vout: u32,
}

impl Ledger for Bitcoin {
    type Quantity = BitcoinQuantity;
    type Address = Address;
    type LockDuration = Blocks;
    type HtlcId = HtlcId;
    type TxId = TransactionId;
    type Pubkey = PublicKey;
    type Identity = PubkeyHash;

    fn symbol() -> String {
        String::from("BTC")
    }
}

impl From<Bitcoin> for swap::Ledger {
    fn from(_: Bitcoin) -> Self {
        swap::Ledger::Bitcoin
    }
}

impl Bitcoin {
    pub fn network(&self) -> Network {
        Network::Regtest
    }
}
