use common_types::seconds::Seconds;
use ethereum_support::{Address, EthereumQuantity, H256};
use ledger_query_service::EthereumQuery;
use secp256k1_support::PublicKey;
use swap_protocols::ledger::Ledger;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Ethereum {}

impl Ledger for Ethereum {
    type Quantity = EthereumQuantity;
    type LockDuration = Seconds;
    type HtlcId = Address;
    type TxId = H256;
    type Pubkey = PublicKey;
    type Address = Address;
    type Identity = Address;
    type QueryForLedgerQueryService = EthereumQuery;

    fn symbol() -> String {
        String::from("ETH")
    }

    fn address_for_identity(&self, address: Address) -> Address {
        address
    }
}
