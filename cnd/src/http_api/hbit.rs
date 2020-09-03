use crate::{
    actions::bitcoin::{BroadcastSignedTransaction, SendToAddress},
    asset,
    bitcoin::Address,
    ledger, Secret, SecretHash, Timestamp,
};
use bitcoin::secp256k1::SecretKey;

pub use crate::hbit::*;

/// Data for the hbit protocol.
#[derive(serde::Deserialize, Clone, Debug)]
pub struct Hbit {
    #[serde(with = "asset::bitcoin::sats_as_string")]
    pub amount: asset::Bitcoin,
    pub final_identity: bitcoin::Address,
    pub network: ledger::Bitcoin,
    pub absolute_expiry: u32,
}

impl From<Hbit> for CreatedSwap {
    fn from(p: Hbit) -> Self {
        CreatedSwap {
            amount: p.amount,
            final_identity: p.final_identity,
            network: p.network,
            absolute_expiry: p.absolute_expiry,
        }
    }
}

/// Data known by the party funding the HTLC in the Hbit protocol, after the
/// swap has been finalized.
///
/// The funder of the HTLC knows the following identities:
/// - `transient_redeem_identity`: the public identity of the redeemer.
/// - `transient_refund_identity`: their own secret identity, from which their
///   public identity can be derived, and which can be used to produce a
///   signature that will enable the refund action.
/// -`final_refund_identity`: the address where the HTLC funds will go if the
///   refund action is executed.
#[derive(Clone, Debug)]
pub struct FinalizedAsFunder {
    pub asset: asset::Bitcoin,
    pub network: ledger::Bitcoin,
    pub transient_redeem_identity: identity::Bitcoin,
    pub final_refund_identity: Address,
    pub transient_refund_identity: SecretKey,
    pub expiry: Timestamp,
    pub state: State,
}

/// Data known by the party redeeming the HTLC in the Hbit protocol, after the
/// swap has been finalized.
///
/// The redeemer of the HTLC knows the following identities:
/// - `transient_refund_identity`: the public identity of the funder.
/// - `transient_redeem_identity`: their own secret identity, from which their
///   public identity can be derived, and which can be used to produce a
///   signature that will enable the redeem action.
/// -`final_refund_identity`: the address where the HTLC funds will go if the
///   redeem action is executed.
#[derive(Clone, Debug)]
pub struct FinalizedAsRedeemer {
    pub asset: asset::Bitcoin,
    pub network: ledger::Bitcoin,
    pub final_redeem_identity: Address,
    pub transient_redeem_identity: SecretKey,
    pub transient_refund_identity: identity::Bitcoin,
    pub expiry: Timestamp,
    pub state: State,
}

impl FinalizedAsFunder {
    pub fn build_fund_action(&self, secret_hash: SecretHash) -> SendToAddress {
        let params = self.build_params(secret_hash);
        params.build_fund_action()
    }

    pub fn build_refund_action(
        &self,
        secret_hash: SecretHash,
    ) -> anyhow::Result<BroadcastSignedTransaction> {
        let (fund_amount, fund_location) = match &self.state {
            State::Funded {
                asset: fund_amount,
                htlc_location: fund_location,
                ..
            } => (fund_amount, fund_location),
            _ => anyhow::bail!("incorrect state"),
        };

        let transient_refund_sk = self.transient_refund_identity;
        let refund_address = self.final_refund_identity.clone().into();
        let params = self.build_params(secret_hash);
        params.build_refund_action(
            &*crate::SECP,
            *fund_amount,
            *fund_location,
            transient_refund_sk,
            refund_address,
        )
    }

    fn build_params(&self, secret_hash: SecretHash) -> Params {
        let transient_refund_sk = self.transient_refund_identity;
        let transient_refund_identity =
            identity::Bitcoin::from_secret_key(&*crate::SECP, &transient_refund_sk);

        Params {
            network: self.network,
            asset: self.asset,
            redeem_identity: self.transient_redeem_identity,
            refund_identity: transient_refund_identity,
            expiry: self.expiry,
            secret_hash,
        }
    }
}

impl FinalizedAsRedeemer {
    pub fn build_redeem_action(
        &self,
        secret: Secret,
    ) -> anyhow::Result<BroadcastSignedTransaction> {
        let (fund_amount, fund_location) = match &self.state {
            State::Funded {
                asset: fund_amount,
                htlc_location: fund_location,
                ..
            } => (fund_amount, fund_location),
            _ => anyhow::bail!("incorrect state"),
        };

        let transient_redeem_sk = self.transient_redeem_identity;
        let redeem_address = self.final_redeem_identity.clone().into();

        let secret_hash = SecretHash::new(secret);
        let params = self.build_params(secret_hash);
        params.build_redeem_action(
            &*crate::SECP,
            *fund_amount,
            *fund_location,
            transient_redeem_sk,
            redeem_address,
            secret,
        )
    }

    fn build_params(&self, secret_hash: SecretHash) -> Params {
        let transient_redeem_sk = self.transient_redeem_identity;
        let transient_redeem_identity =
            identity::Bitcoin::from_secret_key(&*crate::SECP, &transient_redeem_sk);

        Params {
            network: self.network,
            asset: self.asset,
            redeem_identity: transient_redeem_identity,
            refund_identity: self.transient_refund_identity,
            expiry: self.expiry,
            secret_hash,
        }
    }
}
