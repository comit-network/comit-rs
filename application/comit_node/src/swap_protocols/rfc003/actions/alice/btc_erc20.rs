use bitcoin_support::BitcoinQuantity;
use ethereum_support::Erc20Quantity;
use swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        actions::{
            bitcoin::{BitcoinFund, BitcoinRefund},
            ethereum::Erc20Redeem,
            Action, StateActions,
        },
        roles::Alice,
        state_machine::*,
    },
};

impl StateActions for SwapStates<Alice<Bitcoin, Ethereum, BitcoinQuantity, Erc20Quantity>> {
    type Accept = ();
    type Decline = ();
    type Deploy = ();
    type Fund = BitcoinFund;
    type Redeem = Erc20Redeem;
    type Refund = BitcoinRefund;

    fn actions(&self) -> Vec<Action<(), (), (), BitcoinFund, Erc20Redeem, BitcoinRefund>> {
        use self::SwapStates as SS;
        match *self {
            SS::Accepted(Accepted { ref swap, .. }) => vec![Action::Fund(BitcoinFund::new(
                swap.alpha_htlc_params().compute_address(),
                swap.alpha_asset,
            ))],
            SS::BothFunded(BothFunded {
                ref alpha_htlc_location,
                ref beta_htlc_location,
                ref swap,
                ..
            }) => vec![
                Action::Redeem(Erc20Redeem::new(*beta_htlc_location, swap.secret)),
                Action::Refund(BitcoinRefund::new(
                    *alpha_htlc_location,
                    swap.alpha_htlc_params().into(),
                    swap.alpha_asset,
                    swap.alpha_ledger_refund_identity,
                )),
            ],
            SS::AlphaFundedBetaRefunded(AlphaFundedBetaRefunded {
                ref swap,
                ref alpha_htlc_location,
                ..
            })
            | SS::AlphaFundedBetaRedeemed(AlphaFundedBetaRedeemed {
                ref swap,
                ref alpha_htlc_location,
                ..
            }) => vec![Action::Refund(BitcoinRefund::new(
                *alpha_htlc_location,
                swap.alpha_htlc_params().into(),
                swap.alpha_asset,
                swap.alpha_ledger_refund_identity,
            ))],
            SS::AlphaRefundedBetaFunded(AlphaRefundedBetaFunded {
                ref beta_htlc_location,
                ref swap,
                ..
            })
            | SS::AlphaRedeemedBetaFunded(AlphaRedeemedBetaFunded {
                ref beta_htlc_location,
                ref swap,
                ..
            }) => vec![Action::Redeem(Erc20Redeem::new(
                *beta_htlc_location,
                swap.secret,
            ))],
            _ => vec![],
        }
    }
}
