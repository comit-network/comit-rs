use quickcheck::{Arbitrary, Gen};

pub mod secp256k1 {
    use super::*;
    use ::bitcoin::secp256k1::SecretKey;

    pub fn secret_key<G: Gen>(g: &mut G) -> SecretKey {
        let mut bytes = [0u8; 32];
        for byte in &mut bytes {
            *byte = u8::arbitrary(g);
        }
        SecretKey::from_slice(&bytes).unwrap()
    }
}

pub mod bitcoin {
    use super::*;
    use ::bitcoin::Address;

    pub fn address<G: Gen>(g: &mut G) -> Address {
        Address::p2wpkh(
            &crate::identity::Bitcoin::arbitrary(g).into(),
            crate::ledger::Bitcoin::arbitrary(g).into(),
        )
        .unwrap()
    }
}
