use crate::seed::Seed;
use crate::swap_protocols::{rfc003::Secret, SwapId};
use secp256k1_support::KeyPair;

pub trait SecretSource: Send + Sync {
    fn new_secret(&self, id: SwapId) -> Secret;
    fn new_secp256k1_redeem(&self, id: SwapId) -> KeyPair;
    fn new_secp256k1_refund(&self, id: SwapId) -> KeyPair;
}

impl SecretSource for Seed {
    fn new_secret(&self, id: SwapId) -> Secret {
        self.sha256_with_seed(&[id.0.as_bytes(), b"SECRET"]).into()
    }

    fn new_secp256k1_redeem(&self, id: SwapId) -> KeyPair {
        KeyPair::from_secret_key_slice(
            self.sha256_with_seed(&[id.0.as_bytes(), b"REDEEM"])
                .as_ref(),
        )
        .expect("The probability of this happening is < 1 in 2^120")
    }

    fn new_secp256k1_refund(&self, id: SwapId) -> KeyPair {
        KeyPair::from_secret_key_slice(
            self.sha256_with_seed(&[id.0.as_bytes(), b"REFUND"])
                .as_ref(),
        )
        .expect("The probability of this happening is < 1 in 2^120")
    }
}
