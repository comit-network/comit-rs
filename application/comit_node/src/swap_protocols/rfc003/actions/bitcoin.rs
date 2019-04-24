use crate::swap_protocols::{
    ledger::Bitcoin,
    rfc003::{
        actions::OneStepFundActions, bitcoin, secret_source::SecretSource,
        state_machine::HtlcParams, Secret,
    },
};
use bitcoin_support::{BitcoinQuantity, OutPoint};
use bitcoin_witness::PrimedInput;

impl OneStepFundActions<Bitcoin, BitcoinQuantity> for (Bitcoin, BitcoinQuantity) {
    type FundActionOutput = bitcoin::SendToAddress;
    type RefundActionOutput = bitcoin::SpendOutput;
    type RedeemActionOutput = bitcoin::SpendOutput;

    fn fund_action(htlc_params: HtlcParams<Bitcoin, BitcoinQuantity>) -> Self::FundActionOutput {
        let to = htlc_params.compute_address();

        bitcoin::SendToAddress {
            to,
            amount: htlc_params.asset,
            network: htlc_params.ledger.network,
        }
    }

    fn refund_action(
        htlc_params: HtlcParams<Bitcoin, BitcoinQuantity>,
        htlc_location: OutPoint,
        secret_source: &dyn SecretSource,
    ) -> Self::RefundActionOutput {
        let htlc = bitcoin::Htlc::from(htlc_params.clone());

        bitcoin::SpendOutput {
            output: PrimedInput::new(
                htlc_location,
                htlc_params.asset,
                htlc.unlock_after_timeout(secret_source.secp256k1_refund()),
            ),
            network: htlc_params.ledger.network,
        }
    }

    fn redeem_action(
        htlc_params: HtlcParams<Bitcoin, BitcoinQuantity>,
        htlc_location: OutPoint,
        secret_source: &dyn SecretSource,
        secret: Secret,
    ) -> Self::RedeemActionOutput {
        let htlc = bitcoin::Htlc::from(htlc_params.clone());

        bitcoin::SpendOutput {
            output: PrimedInput::new(
                htlc_location,
                htlc_params.asset,
                htlc.unlock_with_secret(secret_source.secp256k1_redeem(), &secret),
            ),
            network: htlc_params.ledger.network,
        }
    }
}
