use crate::{
    asset, identity,
    swap_protocols::{
        actions::bitcoin::{SendToAddress, SpendOutput},
        ledger,
        rfc003::{
            actions::{MakeFundAction, MakeRedeemAction, MakeRefundAction},
            create_swap::HtlcParams,
            DeriveIdentities, Secret,
        },
    },
};
use ::bitcoin::{Amount, OutPoint, Transaction};
use blockchain_contracts::bitcoin::{rfc003::bitcoin_htlc::BitcoinHtlc, witness::PrimedInput};

impl<B> MakeFundAction for (B, asset::Bitcoin)
where
    B: ledger::Bitcoin + ledger::bitcoin::Network,
{
    type HtlcParams = HtlcParams<B, asset::Bitcoin, identity::Bitcoin>;
    type Output = SendToAddress;

    fn make_fund_action(htlc_params: Self::HtlcParams) -> Self::Output {
        let to = htlc_params.compute_address();

        SendToAddress {
            to,
            amount: htlc_params.asset,
            network: B::network(),
        }
    }
}

impl<B> MakeRefundAction for (B, asset::Bitcoin)
where
    B: ledger::Bitcoin + ledger::bitcoin::Network,
{
    type HtlcParams = HtlcParams<B, asset::Bitcoin, identity::Bitcoin>;
    type HtlcLocation = OutPoint;
    type FundTransaction = Transaction;
    type Output = SpendOutput;

    fn make_refund_action(
        htlc_params: Self::HtlcParams,
        htlc_location: Self::HtlcLocation,
        secret_source: &dyn DeriveIdentities,
        fund_transaction: &Self::FundTransaction,
    ) -> Self::Output {
        let htlc = BitcoinHtlc::from(htlc_params);

        SpendOutput {
            output: PrimedInput::new(
                htlc_location,
                Amount::from_sat(fund_transaction.output[htlc_location.vout as usize].value),
                htlc.unlock_after_timeout(&*crate::SECP, secret_source.derive_refund_identity()),
            ),
            network: B::network(),
        }
    }
}

impl<B> MakeRedeemAction for (B, asset::Bitcoin)
where
    B: ledger::Bitcoin + ledger::bitcoin::Network,
{
    type HtlcParams = HtlcParams<B, asset::Bitcoin, identity::Bitcoin>;
    type HtlcLocation = OutPoint;
    type Output = SpendOutput;

    fn make_redeem_action(
        htlc_params: Self::HtlcParams,
        htlc_location: Self::HtlcLocation,
        secret_source: &dyn DeriveIdentities,
        secret: Secret,
    ) -> Self::Output {
        let htlc = BitcoinHtlc::from(htlc_params);

        SpendOutput {
            output: PrimedInput::new(
                htlc_location,
                htlc_params.asset.clone().into(),
                htlc.unlock_with_secret(
                    &*crate::SECP,
                    secret_source.derive_redeem_identity(),
                    secret.into_raw_secret(),
                ),
            ),
            network: B::network(),
        }
    }
}
