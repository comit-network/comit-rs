//! Htlc Bitcoin atomic swap protocol.

use crate::{
    actions::bitcoin::{
        sign_with_fixed_rate, BroadcastSignedTransaction, SendToAddress, SpendOutput,
    },
    asset,
    btsieve::{
        bitcoin::{watch_for_created_outpoint, watch_for_spent_outpoint},
        BlockByHash, LatestBlock,
    },
    htlc_location, identity, ledger,
    timestamp::Timestamp,
    transaction, Secret, SecretHash,
};
use bitcoin::{
    hashes::{hash160, Hash},
    secp256k1::{Secp256k1, SecretKey, Signing},
    Address, Block, BlockHash, Transaction,
};
use blockchain_contracts::bitcoin::{hbit::Htlc, witness::UnlockParameters};
use chrono::{DateTime, Utc};
use futures::{
    future::{self, Either},
    Stream,
};
use genawaiter::sync::{Co, Gen};
use std::cmp::Ordering;
use tracing_futures::Instrument;

/// Represents the events in the hbit protocol.
#[derive(Debug, Clone, PartialEq, strum_macros::Display)]
pub enum Event {
    /// The protocol was started.
    Started,

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

#[derive(Debug, Clone, PartialEq)]
pub struct Redeemed {
    pub transaction: transaction::Bitcoin,
    pub secret: Secret,
}

#[derive(Debug, Clone, PartialEq)]
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
pub fn new<'a, C>(
    connector: &'a C,
    params: Params,
    start_of_swap: DateTime<Utc>,
) -> impl Stream<Item = anyhow::Result<Event>> + 'a
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = BlockHash>,
{
    Gen::new({
        |co| async move {
            if let Err(error) = watch_ledger(connector, params, start_of_swap, &co).await {
                co.yield_(Err(error)).await;
            }
        }
    })
}

async fn watch_ledger<C, R>(
    connector: &C,
    params: Params,
    start_of_swap: DateTime<Utc>,
    co: &Co<anyhow::Result<Event>, R>,
) -> anyhow::Result<()>
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = BlockHash>,
{
    co.yield_(Ok(Event::Started)).await;

    let funded = watch_for_funded(connector, &params, start_of_swap).await?;
    co.yield_(Ok(Event::Funded(funded.clone()))).await;

    let location = match funded {
        Funded::Correctly { location, .. } => location,
        Funded::Incorrectly { location, .. } => location,
    };

    let redeemed = watch_for_redeemed(connector, &params, location, start_of_swap);
    let refunded = watch_for_refunded(connector, &params, location, start_of_swap);

    futures::pin_mut!(redeemed);
    futures::pin_mut!(refunded);

    match future::try_select(redeemed, refunded).await {
        Ok(Either::Left((redeemed, _))) => {
            co.yield_(Ok(Event::Redeemed(redeemed.clone()))).await;
        }
        Ok(Either::Right((refunded, _))) => {
            co.yield_(Ok(Event::Refunded(refunded.clone()))).await;
        }
        Err(either) => {
            let (error, _other_future) = either.factor_first();
            return Err(error);
        }
    }

    Ok(())
}

pub async fn watch_for_funded<C>(
    connector: &C,
    params: &Params,
    start_of_swap: DateTime<Utc>,
) -> anyhow::Result<Funded>
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = BlockHash>,
{
    let expected_asset = params.asset;

    let (transaction, location) =
        watch_for_created_outpoint(connector, start_of_swap, params.compute_address())
            .instrument(tracing::info_span!("", action = "fund"))
            .await?;

    let asset = asset::Bitcoin::from_sat(transaction.output[location.vout as usize].value);

    let event = match expected_asset.cmp(&asset) {
        Ordering::Equal => Funded::Correctly {
            asset,
            transaction,
            location,
        },
        _ => Funded::Incorrectly {
            asset,
            transaction,
            location,
        },
    };

    Ok(event)
}

pub async fn watch_for_redeemed<C>(
    connector: &C,
    params: &Params,
    location: htlc_location::Bitcoin,
    start_of_swap: DateTime<Utc>,
) -> anyhow::Result<Redeemed>
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = BlockHash>,
{
    let (transaction, _) =
        watch_for_spent_outpoint(connector, start_of_swap, location, params.redeem_identity)
            .instrument(tracing::info_span!("", action = "redeem"))
            .await?;

    let secret = extract_secret(&transaction, &params.secret_hash)
        .expect("Redeem transaction must contain secret");

    Ok(Redeemed {
        transaction,
        secret,
    })
}

pub async fn watch_for_refunded<C>(
    connector: &C,
    params: &Params,
    location: htlc_location::Bitcoin,
    start_of_swap: DateTime<Utc>,
) -> anyhow::Result<Refunded>
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = BlockHash>,
{
    let (transaction, _) =
        watch_for_spent_outpoint(connector, start_of_swap, location, params.refund_identity)
            .instrument(tracing::info_span!("", action = "refund"))
            .await?;

    Ok(Refunded { transaction })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Params {
    pub network: ledger::Bitcoin,
    pub asset: asset::Bitcoin,
    pub redeem_identity: identity::Bitcoin,
    pub refund_identity: identity::Bitcoin,
    pub expiry: Timestamp,
    pub secret_hash: SecretHash,
}

impl Params {
    pub fn build_fund_action(&self) -> SendToAddress {
        let network = self.network;
        let to = self.compute_address();
        let amount = self.asset;

        SendToAddress {
            to,
            amount,
            network,
        }
    }

    pub fn build_refund_action<C>(
        &self,
        secp: &Secp256k1<C>,
        fund_amount: asset::Bitcoin,
        fund_location: htlc_location::Bitcoin,
        transient_refund_sk: SecretKey,
        refund_address: Address,
    ) -> anyhow::Result<BroadcastSignedTransaction>
    where
        C: Signing,
    {
        self.build_spend_action(&secp, fund_amount, fund_location, refund_address, |htlc| {
            htlc.unlock_after_timeout(&secp, transient_refund_sk)
        })
    }

    pub fn build_redeem_action<C>(
        &self,
        secp: &Secp256k1<C>,
        fund_amount: asset::Bitcoin,
        fund_location: htlc_location::Bitcoin,
        transient_redeem_sk: SecretKey,
        redeem_address: Address,
        secret: Secret,
    ) -> anyhow::Result<BroadcastSignedTransaction>
    where
        C: Signing,
    {
        self.build_spend_action(&secp, fund_amount, fund_location, redeem_address, |htlc| {
            htlc.unlock_with_secret(secp, transient_redeem_sk, secret.into_raw_secret())
        })
    }

    fn build_spend_action<C>(
        &self,
        secp: &Secp256k1<C>,
        fund_amount: asset::Bitcoin,
        fund_location: htlc_location::Bitcoin,
        spend_address: Address,
        unlock_fn: impl Fn(Htlc) -> UnlockParameters,
    ) -> anyhow::Result<BroadcastSignedTransaction>
    where
        C: Signing,
    {
        let network = self.network;
        let primed_transaction = {
            let htlc = build_bitcoin_htlc(
                self.redeem_identity,
                self.refund_identity,
                self.expiry,
                self.secret_hash,
            );
            let input_parameters = unlock_fn(htlc);
            let spend_output =
                SpendOutput::new(fund_location, fund_amount, input_parameters, network);

            spend_output.spend_to(spend_address)
        };
        let transaction = sign_with_fixed_rate(&secp, primed_transaction)?;

        Ok(BroadcastSignedTransaction {
            transaction,
            network,
        })
    }
}

impl From<Params> for Htlc {
    fn from(params: Params) -> Self {
        build_bitcoin_htlc(
            params.redeem_identity,
            params.refund_identity,
            params.expiry,
            params.secret_hash,
        )
    }
}

impl Params {
    pub fn compute_address(&self) -> Address {
        Htlc::from(*self).compute_address(self.network.into())
    }
}

fn extract_secret(transaction: &Transaction, secret_hash: &SecretHash) -> Option<Secret> {
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

pub fn build_bitcoin_htlc(
    redeem_identity: identity::Bitcoin,
    refund_identity: identity::Bitcoin,
    expiry: Timestamp,
    secret_hash: SecretHash,
) -> Htlc {
    let refund_public_key = ::bitcoin::PublicKey::from(refund_identity);
    let redeem_public_key = ::bitcoin::PublicKey::from(redeem_identity);

    let refund_identity = hash160::Hash::hash(&refund_public_key.key.serialize());
    let redeem_identity = hash160::Hash::hash(&redeem_public_key.key.serialize());

    Htlc::new(
        expiry.into(),
        refund_identity,
        redeem_identity,
        secret_hash.into_raw(),
    )
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
