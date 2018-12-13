use crate::swap_protocols::rfc003::{
    secret::{Secret, SecretHash},
    ExtractSecret,
};
use ethereum_support::Transaction;

impl ExtractSecret for Transaction {
    fn extract_secret(&self, secret_hash: &SecretHash) -> Option<Secret> {
        let data = &self.input.0;
        info!(
            "Attempting to extract secret for {:?} from transaction {:?}",
            secret_hash, self.hash
        );
        match Secret::from_vec(&data) {
            Ok(secret) if secret.hash() == *secret_hash => Some(secret),
            Ok(_) => {
                error!(
                    "Input ({:?}) in transaction {:?} is NOT the pre-image to {:?}",
                    data, self.hash, secret_hash
                );
                None
            }
            Err(e) => {
                error!("Failed to create secret from {:?}: {:?}", data, e);
                None
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use ethereum_support::{Bytes, Transaction, H256, U256};
    use spectral::prelude::*;
    use std::str::FromStr;

    fn setup(secret: &Secret) -> Transaction {
        let _ = pretty_env_logger::try_init();
        Transaction {
            hash: H256::from(123),
            nonce: U256::from(1),
            block_hash: None,
            block_number: None,
            transaction_index: None,
            from: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".parse().unwrap(),
            to: Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".parse().unwrap()),
            value: U256::from(0),
            gas_price: U256::from(0),
            gas: U256::from(0),
            input: Bytes::from(secret.raw_secret().to_vec()),
        }
    }

    #[test]
    fn extract_correct_secret() {
        let secret = Secret::from(*b"This is our favourite passphrase");
        let transaction = setup(&secret);

        assert_that!(transaction.extract_secret(&secret.hash()))
            .is_some()
            .is_equal_to(&secret);
    }

    #[test]
    fn extract_incorrect_secret() {
        let secret = Secret::from(*b"This is our favourite passphrase");
        let transaction = setup(&secret);

        let secret_hash = SecretHash::from_str(
            "bfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbf\
             bfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbf",
        )
        .unwrap();
        assert_that!(transaction.extract_secret(&secret_hash)).is_none();
    }
}
