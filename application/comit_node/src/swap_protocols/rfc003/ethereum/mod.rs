use ethereum_support::{web3::types::Address, Bytes, Erc20Quantity, EtherQuantity};
use hex;
pub use std_ext::time::Seconds;
use swap_protocols::{
    ledger::Ethereum,
    rfc003::{
        secret::{Secret, SecretHash},
        state_machine::HtlcParams,
        Ledger, RedeemTransaction,
    },
};

mod actions;
mod erc20_htlc;
mod ether_htlc;
mod queries;
mod validation;

pub use self::{actions::*, erc20_htlc::*, ether_htlc::*, queries::*};

#[derive(Deserialize, Serialize, Debug)]
pub struct ByteCode(pub String);

impl Into<Bytes> for ByteCode {
    fn into(self) -> Bytes {
        Bytes(hex::decode(self.0).unwrap())
    }
}

pub trait Htlc {
    fn compile_to_hex(&self) -> ByteCode;
}

impl Ledger for Ethereum {
    type LockDuration = Seconds;
    type HtlcLocation = Address;
    type HtlcIdentity = Address;

    fn extract_secret(
        transaction: &RedeemTransaction<Self>,
        secret_hash: &SecretHash,
    ) -> Option<Secret> {
        let transaction = transaction.as_ref();

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

impl From<HtlcParams<Ethereum, EtherQuantity>> for EtherHtlc {
    fn from(htlc_params: HtlcParams<Ethereum, EtherQuantity>) -> Self {
        EtherHtlc::new(
            htlc_params.lock_duration,
            htlc_params.refund_identity,
            htlc_params.success_identity,
            htlc_params.secret_hash,
        )
    }
}

impl HtlcParams<Ethereum, EtherQuantity> {
    pub fn bytecode(&self) -> Bytes {
        EtherHtlc::from(self.clone()).compile_to_hex().into()
    }
}

impl From<HtlcParams<Ethereum, Erc20Quantity>> for Erc20Htlc {
    fn from(htlc_params: HtlcParams<Ethereum, Erc20Quantity>) -> Self {
        Erc20Htlc::new(
            htlc_params.lock_duration,
            htlc_params.refund_identity,
            htlc_params.success_identity,
            htlc_params.secret_hash,
            htlc_params.asset.token_contract(),
            htlc_params.asset.quantity(),
        )
    }
}

impl HtlcParams<Ethereum, Erc20Quantity> {
    pub fn bytecode(&self) -> Bytes {
        Erc20Htlc::from(self.clone()).compile_to_hex().into()
    }
    pub fn funding_tx_payload(&self, htlc_location: Address) -> Bytes {
        Erc20Htlc::from(self.clone())
            .funding_tx_payload(htlc_location)
            .into()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use ethereum_support::{Bytes, Transaction, H256, U256};
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

        assert_that!(Ethereum::extract_secret(
            &RedeemTransaction(transaction),
            &secret.hash()
        ))
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
        assert_that!(Ethereum::extract_secret(
            &RedeemTransaction(transaction),
            &secret_hash
        ))
        .is_none();
    }
}
