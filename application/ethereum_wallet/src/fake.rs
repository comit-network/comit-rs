use InMemoryWallet;
use Wallet;
use hex::FromHex;
use secp256k1::{Secp256k1, SecretKey};
use {SignedTransaction, UnsignedTransaction};

/// A wallet with static private-keys that can be used for testing purposes.
pub struct StaticFakeWallet(InMemoryWallet);

impl Wallet for StaticFakeWallet {
    fn sign<'a>(&self, tx: &'a UnsignedTransaction) -> SignedTransaction<'a> {
        self.0.sign(tx)
    }
}

impl StaticFakeWallet {
    pub fn account0() -> Self {
        let private_key_data = <[u8; 32]>::from_hex(
            "e8aafba2be13ee611059bc756878933bee789cc1aec7c35e23054a44d071c80b",
        ).unwrap();

        let private_key = SecretKey::from_slice(&Secp256k1::new(), &private_key_data).unwrap();
        StaticFakeWallet(InMemoryWallet::new(private_key, 1))
    }
}
