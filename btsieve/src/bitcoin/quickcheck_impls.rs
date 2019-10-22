use crate::quickcheck::Quickcheck;
use bitcoin::hashes::{self, sha256d, Hash};
use quickcheck::{Arbitrary, Gen};

impl Arbitrary for Quickcheck<sha256d::Hash> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let mut bytes = [0u8; 32];
        g.fill_bytes(&mut bytes);

        match sha256d::Hash::from_slice(&bytes) {
            Ok(block_id) => Quickcheck(block_id),
            Err(hashes::Error::InvalidLength(..)) => panic!("we always generate 32 bytes"),
        }
    }
}
