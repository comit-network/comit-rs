use bitcoin::hashes::{sha256d, Hash};
use quickcheck::{Arbitrary, Gen};
use std::ops::Deref;

/// Generic newtype that allows us to implement quickcheck::Arbitrary on foreign
/// types
#[derive(Clone, Debug, Copy)]
pub struct Quickcheck<I>(pub I);

impl<I> Deref for Quickcheck<I> {
    type Target = I;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Arbitrary for Quickcheck<sha256d::Hash> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let bytes = *Quickcheck::<[u8; 32]>::arbitrary(g);

        match sha256d::Hash::from_slice(&bytes) {
            Ok(block_id) => Quickcheck(block_id),
            Err(bitcoin::hashes::Error::InvalidLength(..)) => panic!("we always generate 32 bytes"),
        }
    }
}

impl Arbitrary for Quickcheck<[u8; 32]> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let mut bytes = [0u8; 32];
        g.fill_bytes(&mut bytes);

        Quickcheck(bytes)
    }
}
