use comit_node::swap_protocols::rfc003::{SecretHash, Timestamp};
use crypto::{digest::Digest, sha2::Sha256};
use hex::FromHexError;
use std::{str::FromStr, thread::sleep, time::Duration};

#[derive(Debug)]
pub struct CustomSizeSecret(pub Vec<u8>);

impl CustomSizeSecret {
    pub fn hash(&self) -> SecretHash {
        let mut sha = Sha256::new();
        sha.input(&self.0[..]);

        let mut result: [u8; SecretHash::LENGTH] = [0; SecretHash::LENGTH];
        sha.result(&mut result);
        SecretHash::from(result)
    }
}

impl FromStr for CustomSizeSecret {
    type Err = FromHexError;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        let secret = s.as_bytes().to_vec();
        Ok(CustomSizeSecret(secret))
    }
}

fn diff(first: Timestamp, second: Timestamp) -> u32 {
    u32::from(first).checked_sub(u32::from(second)).unwrap_or(0)
}

pub fn sleep_until(timestamp: Timestamp) {
    let duration = diff(timestamp, Timestamp::now());
    let buffer = 2;

    sleep(Duration::from_secs((duration + buffer).into()));
}
