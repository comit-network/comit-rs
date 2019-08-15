use crate::swap_protocols::{
    actions::bitcoin::{SendToAddress, SpendOutput},
    ledger::Bitcoin,
    rfc003::{
        actions::{FundAction, RedeemAction, RefundAction},
        secret_source::SecretSource,
        state_machine::HtlcParams,
        Secret,
    },
};
use bitcoin_support::{BitcoinQuantity, OutPoint, Transaction};
use bitcoin_witness::PrimedInput;
use blockchain_contracts::bitcoin::rfc003::bitcoin_htlc::BitcoinHtlc;

impl FundAction<Bitcoin, BitcoinQuantity> for (Bitcoin, BitcoinQuantity) {
    type FundActionOutput = SendToAddress;

    fn fund_action(htlc_params: HtlcParams<Bitcoin, BitcoinQuantity>) -> Self::FundActionOutput {
        let to = htlc_params.compute_address();

        SendToAddress {
            to,
            amount: htlc_params.asset,
            network: htlc_params.ledger.network,
        }
    }
}

impl RefundAction<Bitcoin, BitcoinQuantity> for (Bitcoin, BitcoinQuantity) {
    type RefundActionOutput = SpendOutput;

    fn refund_action(
        htlc_params: HtlcParams<Bitcoin, BitcoinQuantity>,
        htlc_location: OutPoint,
        secret_source: &dyn SecretSource,
        fund_transaction: &Transaction,
    ) -> Self::RefundActionOutput {
        let htlc = BitcoinHtlc::from(htlc_params.clone());

        SpendOutput {
            output: PrimedInput::new(
                htlc_location,
                BitcoinQuantity::from_satoshi(
                    fund_transaction.output[htlc_location.vout as usize].value,
                ),
                htlc.unlock_after_timeout(secret_source.secp256k1_refund().secret_key()),
            ),
            network: htlc_params.ledger.network,
        }
    }
}

impl RedeemAction<Bitcoin, BitcoinQuantity> for (Bitcoin, BitcoinQuantity) {
    type RedeemActionOutput = SpendOutput;

    fn redeem_action(
        htlc_params: HtlcParams<Bitcoin, BitcoinQuantity>,
        htlc_location: OutPoint,
        secret_source: &dyn SecretSource,
        secret: Secret,
    ) -> Self::RedeemActionOutput {
        let htlc = BitcoinHtlc::from(htlc_params.clone());

        SpendOutput {
            output: PrimedInput::new(
                htlc_location,
                htlc_params.asset,
                htlc.unlock_with_secret(
                    secret_source.secp256k1_redeem().secret_key(),
                    secret.into_raw_secret(),
                ),
            ),
            network: htlc_params.ledger.network,
        }
    }
}
