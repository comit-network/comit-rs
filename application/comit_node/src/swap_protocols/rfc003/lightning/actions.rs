use bitcoin_support::{BitcoinQuantity, Blocks};
use secp256k1_support::PublicKey;
use swap_protocols::rfc003::{Secret, SecretHash};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AddInvoice {
    pub r_preimage: Secret,
    pub r_hash: SecretHash,
    pub value: BitcoinQuantity,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SendPayment {
    pub dest: PublicKey,
    pub amt: BitcoinQuantity,
    pub payment_hash: SecretHash,
    pub final_cltv_delta: Blocks,
}
