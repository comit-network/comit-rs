use ethereum_support::Transaction;
use swap_protocols::{
    ledger::Ethereum,
    rfc003::{
        secret::{Secret, SecretHash},
        ExtractSecret,
    },
};

impl ExtractSecret for Ethereum {
    fn extract_secret(transaction: &Transaction, secret_hash: &SecretHash) -> Option<Secret> {
        let data = &transaction.input.0;
        info!(
            "Attempting to extract secret for {:?} from transaction {:?}",
            secret_hash, transaction.hash
        );
        match Secret::from_vec(&data) {
            Ok(secret) => match secret.hash() == *secret_hash {
                true => Some(secret),
                false => {
                    error!(
                        "Input ({:?}) in transaction {:?} is NOT the pre-image to {:?}",
                        data, transaction.hash, secret_hash
                    );
                    None
                }
            },
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
    use ethereum_support::{Bytes, H256, U256};
    use pretty_env_logger;
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

        assert_that!(Ethereum::extract_secret(&transaction, &secret.hash()))
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
        assert_that!(Ethereum::extract_secret(&transaction, &secret_hash)).is_none();
    }
}
