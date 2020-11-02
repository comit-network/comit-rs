//! Htlc Bitcoin atomic swap protocol.

use crate::{
    actions::bitcoin::{sign, BroadcastSignedTransaction, SendToAddress, SpendOutput},
    asset,
    btsieve::{
        bitcoin::{watch_for_created_outpoint, watch_for_spent_outpoint},
        BlockByHash, ConnectedNetwork, LatestBlock,
    },
    htlc_location, identity, ledger,
    timestamp::Timestamp,
    Secret, SecretHash,
};
use anyhow::Result;
use bitcoin::{
    hashes::{hash160, Hash},
    secp256k1::{Secp256k1, SecretKey, Signing},
    Address, Block, BlockHash, Transaction,
};
use blockchain_contracts::bitcoin::{hbit::Htlc, witness::UnlockParameters};
use std::cmp::Ordering;
use thiserror::Error;
use time::OffsetDateTime;
use tracing_futures::Instrument;

#[derive(Debug, Clone, Copy, Error)]
#[error("hbit HTLC was incorrectly funded, expected {expected} but got {got}")]
pub struct IncorrectlyFunded {
    pub expected: asset::Bitcoin,
    pub got: asset::Bitcoin,
}

#[derive(Debug, Clone, Copy)]
pub struct Funded {
    pub asset: asset::Bitcoin,
    pub location: htlc_location::Bitcoin,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Redeemed {
    pub transaction: bitcoin::Txid,
    pub secret: Secret,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Refunded {
    pub transaction: bitcoin::Txid,
}

#[async_trait::async_trait]
pub trait WatchForFunded {
    async fn watch_for_funded(
        &self,
        params: &Params,
        start_of_swap: OffsetDateTime,
    ) -> Result<Funded, IncorrectlyFunded>;
}

#[async_trait::async_trait]
pub trait WatchForRedeemed {
    async fn watch_for_redeemed(
        &self,
        params: &Params,
        fund_event: Funded,
        start_of_swap: OffsetDateTime,
    ) -> Redeemed;
}

pub async fn watch_for_funded<C>(
    connector: &C,
    params: &SharedParams,
    start_of_swap: OffsetDateTime,
) -> Result<Result<Funded, IncorrectlyFunded>>
where
    C: LatestBlock<Block = Block>
        + BlockByHash<Block = Block, BlockHash = BlockHash>
        + ConnectedNetwork<Network = ledger::Bitcoin>,
{
    let expected_asset = params.asset;

    let (transaction, location) =
        watch_for_created_outpoint(connector, start_of_swap, params.compute_address())
            .instrument(tracing::info_span!("", action = "fund"))
            .await?;

    let asset = asset::Bitcoin::from_sat(transaction.output[location.vout as usize].value);

    match expected_asset.cmp(&asset) {
        Ordering::Equal => Ok(Ok(Funded { asset, location })),
        _ => Ok(Err(IncorrectlyFunded {
            expected: expected_asset,
            got: asset,
        })),
    }
}

pub async fn watch_for_redeemed<C>(
    connector: &C,
    params: &SharedParams,
    location: htlc_location::Bitcoin,
    start_of_swap: OffsetDateTime,
) -> Result<Redeemed>
where
    C: LatestBlock<Block = Block>
        + BlockByHash<Block = Block, BlockHash = BlockHash>
        + ConnectedNetwork<Network = ledger::Bitcoin>,
{
    let (transaction, _) =
        watch_for_spent_outpoint(connector, start_of_swap, location, params.redeem_identity)
            .instrument(tracing::info_span!("", action = "redeem"))
            .await?;

    let secret = extract_secret(&transaction, &params.secret_hash)
        .expect("Redeem transaction must contain secret");

    Ok(Redeemed {
        transaction: transaction.txid(),
        secret,
    })
}

pub async fn watch_for_refunded<C>(
    connector: &C,
    params: &SharedParams,
    location: htlc_location::Bitcoin,
    start_of_swap: OffsetDateTime,
) -> Result<Refunded>
where
    C: LatestBlock<Block = Block>
        + BlockByHash<Block = Block, BlockHash = BlockHash>
        + ConnectedNetwork<Network = ledger::Bitcoin>,
{
    let (transaction, _) =
        watch_for_spent_outpoint(connector, start_of_swap, location, params.refund_identity)
            .instrument(tracing::info_span!("", action = "refund"))
            .await?;

    Ok(Refunded {
        transaction: transaction.txid(),
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SharedParams {
    pub network: ledger::Bitcoin,
    pub asset: asset::Bitcoin,
    pub redeem_identity: identity::Bitcoin,
    pub refund_identity: identity::Bitcoin,
    pub expiry: Timestamp,
    pub secret_hash: SecretHash,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Params {
    pub shared: SharedParams,
    pub transient_sk: SecretKey,
    pub final_address: bitcoin::Address,
}

impl Params {
    /// Builds the fund action for the hbit protocol.
    pub fn build_fund_action(&self) -> SendToAddress {
        let network = self.shared.network;
        let to = self.shared.compute_address();
        let amount = self.shared.asset;

        SendToAddress {
            to,
            amount,
            network,
        }
    }

    /// Builds the refund action for the hbit protocol.
    ///
    /// This function assumes that the HTLC was funded with the intended amount.
    /// Be aware that if that is not the case, then this function might result
    /// in absurdly high fees because it spends only the originally intended
    /// amount.
    pub fn build_refund_action<C>(
        &self,
        secp: &Secp256k1<C>,
        fund_location: htlc_location::Bitcoin,
        vbyte_rate: asset::Bitcoin,
    ) -> Result<BroadcastSignedTransaction>
    where
        C: Signing,
    {
        self.build_spend_action(
            &secp,
            self.shared.asset,
            fund_location,
            self.final_address.clone(),
            vbyte_rate,
            |htlc, secret_key| htlc.unlock_after_timeout(&secp, secret_key),
        )
    }

    /// Builds the redeem action for the hbit protocol.
    ///
    /// This function assumes that the HTLC was funded with the intended amount.
    /// Be aware that if that is not the case, then this function might result
    /// in absurdly high fees because it spends only the originally intended
    /// amount.
    pub fn build_redeem_action<C>(
        &self,
        secp: &Secp256k1<C>,
        fund_location: htlc_location::Bitcoin,
        secret: Secret,
        vbyte_rate: asset::Bitcoin,
    ) -> Result<BroadcastSignedTransaction>
    where
        C: Signing,
    {
        self.build_spend_action(
            &secp,
            self.shared.asset,
            fund_location,
            self.final_address.clone(),
            vbyte_rate,
            |htlc, secret_key| htlc.unlock_with_secret(secp, secret_key, secret.into_raw_secret()),
        )
    }

    pub fn build_spend_action<C>(
        &self,
        secp: &Secp256k1<C>,
        fund_amount: asset::Bitcoin,
        fund_location: htlc_location::Bitcoin,
        spend_address: Address,
        vbyte_rate: asset::Bitcoin,
        unlock_fn: impl Fn(Htlc, SecretKey) -> UnlockParameters,
    ) -> Result<BroadcastSignedTransaction>
    where
        C: Signing,
    {
        let network = self.shared.network;
        let primed_transaction = {
            let htlc = self.shared.into();
            let input_parameters = unlock_fn(htlc, self.transient_sk);
            let spend_output =
                SpendOutput::new(fund_location, fund_amount, input_parameters, network);

            spend_output.spend_to(spend_address)
        };
        let transaction = sign(&secp, primed_transaction, vbyte_rate)?;

        Ok(BroadcastSignedTransaction {
            transaction,
            network,
        })
    }
}

impl From<SharedParams> for Htlc {
    fn from(params: SharedParams) -> Self {
        build_bitcoin_htlc(
            params.redeem_identity,
            params.refund_identity,
            params.expiry,
            params.secret_hash,
        )
    }
}

impl SharedParams {
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

#[cfg(feature = "quickcheck")]
mod arbitrary {
    use super::*;
    use crate::{asset, identity, ledger, SecretHash, Timestamp};
    use ::bitcoin::secp256k1::SecretKey;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for Params {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Params {
                shared: SharedParams {
                    network: bitcoin_network(g),
                    asset: bitcoin_asset(g),
                    redeem_identity: bitcoin_identity(g),
                    refund_identity: bitcoin_identity(g),
                    expiry: Timestamp::arbitrary(g),
                    secret_hash: SecretHash::arbitrary(g),
                },
                transient_sk: secret_key(g),
                final_address: bitcoin_address(g),
            }
        }
    }

    fn secret_key<G: Gen>(g: &mut G) -> SecretKey {
        let mut bytes = [0u8; 32];
        for byte in &mut bytes {
            *byte = u8::arbitrary(g);
        }
        SecretKey::from_slice(&bytes).unwrap()
    }

    fn bitcoin_network<G: Gen>(g: &mut G) -> ledger::Bitcoin {
        match u8::arbitrary(g) % 3 {
            0 => ledger::Bitcoin::Mainnet,
            1 => ledger::Bitcoin::Testnet,
            2 => ledger::Bitcoin::Regtest,
            _ => unreachable!(),
        }
    }

    fn bitcoin_asset<G: Gen>(g: &mut G) -> asset::Bitcoin {
        asset::Bitcoin::from_sat(u64::arbitrary(g))
    }

    fn bitcoin_identity<G: Gen>(g: &mut G) -> identity::Bitcoin {
        identity::Bitcoin::from_secret_key(
            &bitcoin::secp256k1::Secp256k1::signing_only(),
            &secret_key(g),
        )
    }

    fn bitcoin_address<G: Gen>(g: &mut G) -> bitcoin::Address {
        bitcoin::Address::p2wpkh(&bitcoin_identity(g).into(), bitcoin_network(g).into()).unwrap()
    }
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
