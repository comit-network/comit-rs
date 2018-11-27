use bitcoin_support::BitcoinQuantity;
use ethereum_support::Erc20Quantity;
use swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        actions::{
            bitcoin::BitcoinRedeem,
            ethereum::{Erc20Deploy, Erc20Fund, Erc20Refund},
            Action, StateActions,
        },
        roles::Alice,
        state_machine::*,
    },
};

impl StateActions for SwapStates<Alice<Ethereum, Bitcoin, Erc20Quantity, BitcoinQuantity>> {
    type Accept = ();
    type Decline = ();
    type Deploy = Erc20Deploy;
    type Fund = Erc20Fund;
    type Redeem = BitcoinRedeem;
    type Refund = Erc20Refund;

    fn actions(&self) -> Vec<Action<(), (), Erc20Deploy, Erc20Fund, BitcoinRedeem, Erc20Refund>> {
        use self::SwapStates as SS;
        match *self {
            SS::Accepted(Accepted { ref swap, .. }) => vec![Action::Deploy(Erc20Deploy::new(
                swap.alpha_htlc_params().bytecode(),
            ))],
            SS::AlphaDeployed { .. } => vec![], // TODO: Add Fund Action
            SS::BothFunded(BothFunded {
                ref alpha_htlc_location,
                ref beta_htlc_location,
                ref swap,
                ..
            }) => vec![
                Action::Redeem(BitcoinRedeem::new(
                    *beta_htlc_location,
                    swap.beta_htlc_params().into(),
                    swap.beta_asset,
                    swap.beta_ledger_success_identity,
                    swap.secret,
                )),
                Action::Refund(Erc20Refund::new(*alpha_htlc_location)),
            ],
            SS::AlphaFundedBetaRefunded(AlphaFundedBetaRefunded {
                ref alpha_htlc_location,
                ..
            })
            | SS::AlphaFundedBetaRedeemed(AlphaFundedBetaRedeemed {
                ref alpha_htlc_location,
                ..
            }) => vec![Action::Refund(Erc20Refund::new(*alpha_htlc_location))],
            SS::AlphaRefundedBetaFunded(AlphaRefundedBetaFunded {
                ref beta_htlc_location,
                ref swap,
                ..
            })
            | SS::AlphaRedeemedBetaFunded(AlphaRedeemedBetaFunded {
                ref beta_htlc_location,
                ref swap,
                ..
            }) => vec![Action::Redeem(BitcoinRedeem::new(
                *beta_htlc_location,
                swap.beta_htlc_params().into(),
                swap.beta_asset,
                swap.beta_ledger_success_identity,
                swap.secret,
            ))],
            _ => vec![],
        }
    }
}
