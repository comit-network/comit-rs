use crypto::{digest::Digest, sha2::Sha256};
use hex_serde;
use std::fmt;

pub const SEED_LENGTH: usize = 32;
#[derive(Clone, Copy, Deserialize)]
pub struct Seed(#[serde(with = "hex_serde")] [u8; SEED_LENGTH]);

impl fmt::Debug for Seed {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Seed")
    }
}

impl Seed {
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
}
