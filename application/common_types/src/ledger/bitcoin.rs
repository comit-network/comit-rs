use bitcoin_rpc::Address;
use bitcoin_support::BitcoinQuantity;
use ledger::Ledger;

#[derive(Clone, Debug)]
pub struct Bitcoin {}

impl Ledger for Bitcoin {
    type Quantity = BitcoinQuantity;
    type Address = Address;

    fn symbol() -> String {
        String::from("BTC")
    }
}
