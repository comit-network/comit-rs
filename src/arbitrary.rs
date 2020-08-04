use comit::{SecretHash, Timestamp};
use quickcheck::{Arbitrary, Gen};

pub fn secret_hash<G: Gen>(g: &mut G) -> SecretHash {
    let mut bytes = [0u8; 32];
    for byte in &mut bytes {
        *byte = u8::arbitrary(g);
    }
    SecretHash::from(bytes)
}

pub fn timestamp<G: Gen>(g: &mut G) -> Timestamp {
    Timestamp::from(u32::arbitrary(g))
}
