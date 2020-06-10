use bitcoin::secp256k1::SecretKey;
use comit::{
    actions::bitcoin::{BroadcastSignedTransaction, SendToAddress, SpendOutput},
    asset, ledger, Secret, SecretHash, Timestamp,
};

pub use crate::{actions::bitcoin::sign_with_fixed_rate, hbit::*};

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
    pub final_refund_identity: comit::bitcoin::Address,
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
    pub final_redeem_identity: comit::bitcoin::Address,
    pub transient_redeem_identity: SecretKey,
    pub transient_refund_identity: identity::Bitcoin,
    pub expiry: Timestamp,
    pub state: State,
}

impl FinalizedAsFunder {
    pub fn build_fund_action(&self, secret_hash: SecretHash) -> SendToAddress {
        let transient_refund_sk = self.transient_refund_identity;
        let transient_refund_identity =
            identity::Bitcoin::from_secret_key(&*crate::SECP, &transient_refund_sk);
        let htlc = build_bitcoin_htlc(
            self.transient_redeem_identity,
            transient_refund_identity,
            self.expiry,
            secret_hash,
        );
        let network = bitcoin::Network::from(self.network);
        let to = htlc.compute_address(network);
        let amount = self.asset;

        SendToAddress {
            to,
            amount,
            network,
        }
    }

    pub fn build_refund_action(
        &self,
        secret_hash: SecretHash,
    ) -> anyhow::Result<BroadcastSignedTransaction> {
        let (htlc_location, fund_transaction) = match &self.state {
            State::Funded {
                htlc_location,
                fund_transaction,
                ..
            } => (htlc_location, fund_transaction),
            _ => anyhow::bail!("incorrect state"),
        };

        let network = bitcoin::Network::from(self.network);
        let spend_output = {
            let transient_refund_sk = self.transient_refund_identity;
            let transient_refund_identity =
                identity::Bitcoin::from_secret_key(&*crate::SECP, &transient_refund_sk);
            let htlc = build_bitcoin_htlc(
                self.transient_redeem_identity,
                transient_refund_identity,
                self.expiry,
                secret_hash,
            );

            let previous_output = htlc_location;
            let value = bitcoin::Amount::from_sat(
                fund_transaction.output[htlc_location.vout as usize].value,
            );
            let input_parameters = htlc.unlock_after_timeout(&*crate::SECP, transient_refund_sk);

            SpendOutput::new(*previous_output, value, input_parameters, network)
        };

        let primed_transaction = spend_output.spend_to(self.final_refund_identity.clone().into());
        let transaction = sign_with_fixed_rate(&*crate::SECP, primed_transaction)?;

        Ok(BroadcastSignedTransaction {
            transaction,
            network,
        })
    }
}

impl FinalizedAsRedeemer {
    pub fn build_redeem_action(
        &self,
        secret: Secret,
    ) -> anyhow::Result<BroadcastSignedTransaction> {
        let (htlc_location, fund_transaction) = match &self.state {
            State::Funded {
                htlc_location,
                fund_transaction,
                ..
            } => (htlc_location, fund_transaction),
            _ => anyhow::bail!("incorrect state"),
        };

        let network = bitcoin::Network::from(self.network);
        let spend_output = {
            let transient_redeem_sk = self.transient_redeem_identity;
            let transient_redeem_identity =
                identity::Bitcoin::from_secret_key(&*crate::SECP, &transient_redeem_sk);
            let htlc = build_bitcoin_htlc(
                transient_redeem_identity,
                self.transient_refund_identity,
                self.expiry,
                SecretHash::new(secret),
            );

            let previous_output = htlc_location;
            let value = bitcoin::Amount::from_sat(
                fund_transaction.output[htlc_location.vout as usize].value,
            );
            let input_parameters = htlc.unlock_with_secret(
                &*crate::SECP,
                transient_redeem_sk,
                secret.into_raw_secret(),
            );

            SpendOutput::new(*previous_output, value, input_parameters, network)
        };

        let primed_transaction = spend_output.spend_to(self.final_redeem_identity.clone().into());
        let transaction = sign_with_fixed_rate(&*crate::SECP, primed_transaction)?;

        Ok(BroadcastSignedTransaction {
            transaction,
            network,
        })
    }
}
