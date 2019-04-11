use crate::seed::Seed;
use blockchain_contracts::rfc003::secret::Secret;
use secp256k1_support::KeyPair;

pub trait SecretSource: Send + Sync {
    fn secret(&self) -> Secret;
    fn secp256k1_redeem(&self) -> KeyPair;
    fn secp256k1_refund(&self) -> KeyPair;
}

impl SecretSource for Seed {
    fn secret(&self) -> Secret {
        self.sha256_with_seed(&[b"SECRET"]).into()
    }

    fn secp256k1_redeem(&self) -> KeyPair {
        KeyPair::from_secret_key_slice(self.sha256_with_seed(&[b"REDEEM"]).as_ref())
            .expect("The probability of this happening is < 1 in 2^120")
    }

    fn secp256k1_refund(&self) -> KeyPair {
        KeyPair::from_secret_key_slice(self.sha256_with_seed(&[b"REFUND"]).as_ref())
            .expect("The probability of this happening is < 1 in 2^120")
    }
}
