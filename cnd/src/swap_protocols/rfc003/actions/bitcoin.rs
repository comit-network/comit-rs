use crate::swap_protocols::{
    actions::bitcoin::{SendToAddress, SpendHtlc},
    ledger::Bitcoin,
    rfc003::{
        actions::{FundAction, RedeemAction, RefundAction},
        secret_source::SecretSource,
        state_machine::HtlcParams,
        Secret,
    },
};
use bitcoin_support::{BitcoinQuantity, OutPoint, Transaction};
use blockchain_contracts::bitcoin::rfc003::BitcoinHtlc;

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
    type RefundActionOutput = SpendHtlc;

    fn refund_action(
        htlc_params: HtlcParams<Bitcoin, BitcoinQuantity>,
        htlc_location: OutPoint,
        secret_source: &dyn SecretSource,
        fund_transaction: &Transaction,
    ) -> Self::RefundActionOutput {
        SpendHtlc {
            htlc: BitcoinHtlc::from(htlc_params.clone()),
            outpoint: htlc_location,
            amount: BitcoinQuantity::from_satoshi(
                fund_transaction.output[htlc_location.vout as usize].value,
            ),
            key: secret_source.secp256k1_refund().secret_key(),
            secret: None,
            network: htlc_params.ledger.network,
        }
    }
}

impl RedeemAction<Bitcoin, BitcoinQuantity> for (Bitcoin, BitcoinQuantity) {
    type RedeemActionOutput = SpendHtlc;

    fn redeem_action(
        htlc_params: HtlcParams<Bitcoin, BitcoinQuantity>,
        htlc_location: OutPoint,
        secret_source: &dyn SecretSource,
        secret: Secret,
    ) -> Self::RedeemActionOutput {
        let HtlcParams { asset, ledger, .. } = htlc_params.clone();

        SpendHtlc {
            htlc: BitcoinHtlc::from(htlc_params),
            outpoint: htlc_location,
            amount: asset,
            key: secret_source.secp256k1_redeem().secret_key(),
            secret: Some(secret),
            network: ledger.network,
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn refund_transfers_everything_from_bitcoin_htlc() {
        unimplemented!()
    }
}
