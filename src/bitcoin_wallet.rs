use ::bitcoin::secp256k1::constants::SECRET_KEY_SIZE;
use rand::prelude::*;

struct Seed([u8; SECRET_KEY_SIZE]);

impl Seed {
    pub fn new() -> Self {
        let mut bytes = [0u8; SECRET_KEY_SIZE];

        rand::thread_rng().fill_bytes(&mut bytes);
        Seed(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_random_seed() {
        let _seed = Seed::new();
    }
}
