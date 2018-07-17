use hex::FromHex;

use secp256k1_support::KeyPair;
use InMemoryWallet;
use SignedTransaction;
use UnsignedTransaction;
use Wallet;

/// A wallet with static private-keys that can be used for testing purposes.
pub struct StaticFakeWallet(InMemoryWallet);

impl Wallet for StaticFakeWallet {
    fn sign<'a>(&self, tx: &'a UnsignedTransaction) -> SignedTransaction<'a> {
        self.0.sign(tx)
    }
}

impl StaticFakeWallet {
    pub fn account0() -> Self {
        let secret_key_data = <[u8; 32]>::from_hex(
            "e8aafba2be13ee611059bc756878933bee789cc1aec7c35e23054a44d071c80b",
        ).unwrap();

        let keypair = KeyPair::from_secret_key_slice(&secret_key_data).unwrap();
        StaticFakeWallet(InMemoryWallet::new(keypair, 1))
    }
}
