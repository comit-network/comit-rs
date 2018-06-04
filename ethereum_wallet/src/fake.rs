use InMemoryWallet;
use Transaction;
use Wallet;
use hex::FromHex;
use web3::types::Bytes;

/// A wallet with static private-keys that can be used for testing purposes.
pub struct StaticFakeWallet(InMemoryWallet);

impl Wallet for StaticFakeWallet {
    fn create_signed_raw_transaction(&self, tx: &Transaction) -> Bytes {
        self.0.create_signed_raw_transaction(tx)
    }
}

impl StaticFakeWallet {
    pub fn account0() -> Self {
        let private_key = <[u8; 32]>::from_hex(
            "e8aafba2be13ee611059bc756878933bee789cc1aec7c35e23054a44d071c80b",
        ).unwrap();

        StaticFakeWallet(InMemoryWallet::new(private_key, 1).unwrap())
    }
}
