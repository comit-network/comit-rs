use crate::swap_protocols::SwapId;
use crypto::{digest::Digest, sha2::Sha256};
use rand::{rngs::OsRng, Rng};
use serde::{Deserialize, Serialize};
use std::fmt;

pub const SEED_LENGTH: usize = 32;
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Seed(#[serde(with = "hex_serde")] [u8; SEED_LENGTH]);

impl fmt::Debug for Seed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Seed([*****])")
    }
}

impl fmt::Display for Seed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Seed {
    pub fn swap_seed(&self, id: SwapId) -> Seed {
        Seed(self.sha256_with_seed(&[b"SWAP", id.0.as_bytes()]))
    }
    pub fn sha256_with_seed(&self, slices: &[&[u8]]) -> [u8; SEED_LENGTH] {
        let mut sha = Sha256::new();
        sha.input(&self.0);
        for slice in slices {
            sha.input(slice);
        }
        let mut result = [0u8; SEED_LENGTH];
        sha.result(&mut result);
        result
    }

    pub fn new_random() -> Result<Seed, rand::Error> {
        let mut rng = OsRng::new()?;
        let mut arr = [0u8; 32];
        rng.try_fill(&mut arr[..])?;
        Ok(Seed(arr))
    }
}

impl From<[u8; 32]> for Seed {
    fn from(seed: [u8; 32]) -> Self {
        Seed(seed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn data_and_seed_used_to_calculate_hash() {
        let seed1 = Seed::from(*b"hello world, you are beautiful!!");
        assert_ne!(
            seed1.sha256_with_seed(&[b"foo"]),
            seed1.sha256_with_seed(&[b"bar"])
        );

        let seed2 = Seed::from(*b"bye world, you are beautiful!!!!");
        assert_ne!(
            seed1.sha256_with_seed(&[b"foo"]),
            seed2.sha256_with_seed(&[b"foo"])
        );
    }

    #[test]
    fn test_two_random_seeds_are_different() {
        let random1 = Seed::new_random().unwrap();
        let random2 = Seed::new_random().unwrap();

        assert_ne!(random1, random2);
    }

    #[test]
    fn test_display_and_debug_not_implemented() {
        let seed = Seed::new_random().unwrap();

        let out = format!("{}", seed);
        assert_eq!(out, "Seed([*****])".to_string());
        let debug = format!("{:?}", seed);
        assert_eq!(debug, "Seed([*****])".to_string());
    }
}
