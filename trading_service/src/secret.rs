use crypto::digest::Digest;
use crypto::sha2::Sha256;
use rand::{OsRng, Rng};

const SHA256_DIGEST_LENGTH: usize = 32;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SecretHash(pub Vec<u8>);

impl SecretHash {
    pub fn as_hex(&self) -> String {
        let mut s = String::new();
        for i in &self.0 {
            // 02x -> always output 2 chars, left pad with zero if needed
            s.push_str(&format!("{:02x}", i));
        }
        s
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Secret {
    secret: Vec<u8>,
    hash: Option<SecretHash>,
}

impl Secret {
    pub fn generate<T: RandomnessSource>(rng: &mut T) -> Secret {
        let secret = rng.gen_random_bytes(SHA256_DIGEST_LENGTH);
        Secret::new(secret)
    }

    pub fn new(secret: Vec<u8>) -> Secret {
        Secret { secret, hash: None }
    }

    pub fn hash(&mut self) -> &SecretHash {
        match self.hash {
            None => {
                let mut sha = Sha256::new();
                sha.input(self.secret.as_slice());

                let mut result: [u8; SHA256_DIGEST_LENGTH] = [0; SHA256_DIGEST_LENGTH];
                sha.result(&mut result);
                let hash = SecretHash(result.to_vec());

                self.hash = Some(hash.clone());
                self.hash()
            }
            Some(ref hash) => hash,
        }
    }
}

pub trait RandomnessSource {
    fn gen_random_bytes(&mut self, nbytes: usize) -> Vec<u8>;
}

impl RandomnessSource for OsRng {
    fn gen_random_bytes(&mut self, nbytes: usize) -> Vec<u8> {
        let mut buf: Vec<u8> = vec![0; nbytes];
        self.fill_bytes(&mut buf);
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex;
    use std::vec::Vec;

    #[test]
    fn gen_random_bytes_not_zeros() {
        let mut rng = OsRng::new().unwrap();

        let empty_buf: Vec<u8> = vec![0; 32];
        let buf = rng.gen_random_bytes(32);
        assert_eq!(buf.len(), 32);
        assert_ne!(buf, empty_buf);
    }

    #[test]
    fn new_secret_hash() {
        let bytes: Vec<u8> = b"hello world, you are beautiful!!".to_vec();
        let mut secret = Secret::new(bytes);
        assert_eq!(
            *secret.hash(),
            SecretHash(
                hex::decode("68d627971643a6f97f27c58957826fcba853ec2077fd10ec6b93d8e61deb4cec")
                    .unwrap()
            )
        );
    }

    #[test]
    fn new_secret_hash_as_hex() {
        let bytes: Vec<u8> = b"hello world, you are beautiful!!".to_vec();
        let mut secret = Secret::new(bytes);
        assert_eq!(
            secret.hash().as_hex(),
            "68d627971643a6f97f27c58957826fcba853ec2077fd10ec6b93d8e61deb4cec"
        );
    }
}
