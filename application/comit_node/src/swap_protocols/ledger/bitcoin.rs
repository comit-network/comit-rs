use bitcoin_rpc_client::TransactionId;
use bitcoin_support::{Address, BitcoinQuantity, Blocks, Network, PubkeyHash};
use secp256k1_support::PublicKey;
use swap_protocols::ledger::Ledger;

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

impl Bitcoin {
    pub fn network(&self) -> Network {
        Network::Regtest
    }
}
