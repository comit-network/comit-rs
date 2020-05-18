//! Htlc Bitcoin atomic swap protocol.

use crate::{
    asset, htlc_location, identity, timestamp::Timestamp, transaction, Secret, SecretHash,
};
use bitcoin::Transaction;
use blockchain_contracts::bitcoin::rfc003::bitcoin_htlc::BitcoinHtlc;
use chrono::NaiveDateTime;
use futures::{
    future::{self, Either},
    Stream,
};
use genawaiter::sync::{Co, Gen};

/// Data required to create a swap that involves Bitcoin.
#[derive(Clone, Copy, Debug)]
pub struct CreatedSwap {
    pub amount: asset::Bitcoin,
    pub identity: identity::Bitcoin,
    pub network: ledger::Bitcoin,
    pub absolute_expiry: u32,
}

/// Resolves when said event has occurred.

#[async_trait::async_trait]
pub trait WaitForFunded {
    async fn wait_for_funded(
        &self,
        params: Params,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Funded>;
}

#[async_trait::async_trait]
pub trait WaitForRedeemed {
    async fn wait_for_redeemed(
        &self,
        params: Params,
        funded: Funded,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Redeemed>;
}

#[async_trait::async_trait]
pub trait WaitForRefunded {
    async fn wait_for_refunded(
        &self,
        params: Params,
        funded: Funded,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Refunded>;
}

/// Represents the events in the hbit protocol.
#[derive(Debug, Clone, PartialEq, strum_macros::Display)]
pub enum Event {
    /// The HTLC has been funded with bitcoin.
    Funded(Funded),

    /// The HTLC has been destroyed via the redeem path, bitcoin have been sent
    /// to the redeemer.
    Redeemed(Redeemed),

    /// The HTLC has been destroyed via the refund path, bitcoin has been sent
    /// back to funder.
    Refunded(Refunded),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Funded {
    Correctly {
        asset: asset::Bitcoin,
        transaction: transaction::Bitcoin,
        location: htlc_location::Bitcoin,
    },
    Incorrectly {
        asset: asset::Bitcoin,
        transaction: transaction::Bitcoin,
        location: htlc_location::Bitcoin,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Redeemed {
    pub transaction: transaction::Bitcoin,
    pub secret: Secret,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Refunded {
    pub transaction: transaction::Bitcoin,
}

/// Creates a new instance of the hbit protocol.
///
/// Returns a stream of events happening during the execution.
///
/// The current implementation is naive in the sense that it does not take into
/// account situations where it is clear that no more events will happen even
/// though in theory, there could. For example:
/// - funded
/// - refunded
///
/// It is highly unlikely for Bob to fund the HTLC now, yet the current
/// implementation is still waiting for that.
pub fn new(
    connector: Arc<Cache<BitcoindConnector>>,
    params: Params,
) -> impl Stream<Item = anyhow::Result<Event>> {
    Gen::new({
        |co| async move {
            if let Err(error) = watch_ledger(connector, params, &co).await {
                co.yield_(Err(error)).await;
            }
        }
    })
}

/// Returns a future that waits for events to happen on a ledger.
///
/// Each event is yielded through the controller handle (co) of the coroutine.
async fn watch_ledger(
    connector: Arc<Cache<BitcoindConnector>>,
    params: Params,
    co: &Co<anyhow::Result<Event>>,
) -> anyhow::Result<()> {
    let funded = connector.htlc_funded(&params, start_of_swap).await?;
    co.yield_(Event::Funded(funded)).await;

    let redeemed = connector.htlc_redeemed(&params, &funded, start_of_swap);
    let refunded = connector.htlc_refunded(&params, &funded, start_of_swap);

    match future::try_select(redeemed, refunded).await {
        Ok(Either::Left((redeemed, _))) => {
            co.yield_(Event::Redeemed(redeemed.clone())).await;
        }
        Ok(Either::Right((refunded, _))) => {
            co.yield_(Event::Refunded(refunded.clone())).await;
        }
        Err(either) => {
            let (error, _other_future) = either.factor_first();
            return Err(error);
        }
    }

    Ok(())
}

#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub network: bitcoin::Network,
    pub asset: asset::Bitcoin,
    pub redeem_identity: identity::Bitcoin,
    pub refund_identity: identity::Bitcoin,
    pub expiry: Timestamp,
    pub secret_hash: SecretHash,
}

impl From<Params> for BitcoinHtlc {
    fn from(htlc_params: Params) -> Self {
        let refund_public_key = ::bitcoin::PublicKey::from(htlc_params.refund_identity);
        let redeem_public_key = ::bitcoin::PublicKey::from(htlc_params.redeem_identity);

        let refund_identity = hash160::Hash::hash(&refund_public_key.key.serialize());
        let redeem_identity = hash160::Hash::hash(&redeem_public_key.key.serialize());

        BitcoinHtlc::new(
            htlc_params.expiry.into(),
            refund_identity,
            redeem_identity,
            htlc_params.secret_hash.into_raw(),
        )
    }
}

impl Params {
    pub fn compute_address(&self) -> Address {
        BitcoinHtlc::from(*self).compute_address(self.network)
    }
}
pub fn extract_secret(transaction: &Transaction, secret_hash: &SecretHash) -> Option<Secret> {
    transaction.input.iter().find_map(|txin| {
        txin.witness
            .iter()
            .find_map(|script_item| match Secret::from_vec(&script_item) {
                Ok(secret) if SecretHash::new(secret) == *secret_hash => Some(secret),
                Ok(_) => None,
                Err(_) => None,
            })
    })
}

#[cfg(test)]
mod test {
    use super::*;
    use bitcoin::{consensus::encode::deserialize, OutPoint, Script, Transaction, TxIn};
    use spectral::prelude::*;
    use std::str::FromStr;

    fn setup(secret: &Secret) -> Transaction {
        Transaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: OutPoint::null(),
                script_sig: Script::new(),
                sequence: 0,
                witness: vec![
                    vec![],                          // Signature
                    vec![],                          // Public key
                    secret.as_raw_secret().to_vec(), // Secret
                    vec![1u8],                       // Bool to enter redeem branch
                    vec![],                          // Previous Script
                ],
            }],
            output: vec![],
        }
    }

    #[test]
    fn extract_correct_secret() {
        let secret = Secret::from(*b"This is our favourite passphrase");
        let transaction = setup(&secret);

        assert_that!(extract_secret(&transaction, &SecretHash::new(secret)))
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
        assert_that!(extract_secret(&transaction, &secret_hash)).is_none();
    }

    #[test]
    fn extract_correct_secret_from_mainnet_transaction() {
        let hex_tx = hex::decode("0200000000010124e06fe5594b941d06c7385dc7307ec694a41f7d307423121855ee17e47e06ad0100000000ffffffff0137aa0b000000000017a914050377baa6e8c5a07aed125d0ef262c6d5b67a038705483045022100d780139514f39ed943179e4638a519101bae875ec1220b226002bcbcb147830b0220273d1efb1514a77ee3dd4adee0e896b7e76be56c6d8e73470ae9bd91c91d700c01210344f8f459494f74ebb87464de9b74cdba3709692df4661159857988966f94262f20ec9e9fb3c669b2354ea026ab3da82968a2e7ab9398d5cbed4e78e47246f2423e01015b63a82091d6a24697ed31932537ae598d3de3131e1fcd0641b9ac4be7afcb376386d71e8876a9149f4a0cf348b478336cb1d87ea4c8313a7ca3de1967029000b27576a91465252e57f727a27f32c77098e14d88d8dbec01816888ac00000000").unwrap();
        let transaction: Transaction = deserialize(&hex_tx).unwrap();
        let hex_secret =
            hex::decode("ec9e9fb3c669b2354ea026ab3da82968a2e7ab9398d5cbed4e78e47246f2423e")
                .unwrap();
        let secret = Secret::from_vec(&hex_secret).unwrap();

        assert_that!(extract_secret(&transaction, &SecretHash::new(secret)))
            .is_some()
            .is_equal_to(&secret);
    }
}
