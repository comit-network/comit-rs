use crate::quickcheck::Quickcheck;
use bitcoin_support::{BlockId, Hash, HashesError};
use quickcheck::{Arbitrary, Gen};

impl Arbitrary for Quickcheck<BlockId> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let mut bytes = [0u8; 32];
        g.fill_bytes(&mut bytes);

        match BlockId::from_slice(&bytes) {
            Ok(block_id) => Quickcheck(block_id),
            Err(HashesError::InvalidLength(..)) => panic!("we always generate 32 bytes"),
        }
    }
}
