use secp256k1_support::PublicKey;
use swap_protocols::ledger::Ledger;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Lightning {}
type PaymentHash = String;

impl Ledger for Lightning {
    type TxId = PaymentHash;
    type Pubkey = PublicKey;
    type Address = PublicKey;
    type Identity = PublicKey;
    type Transaction = ();

    fn address_for_identity(&self, public_key: PublicKey) -> PublicKey {
        public_key
    }
}
