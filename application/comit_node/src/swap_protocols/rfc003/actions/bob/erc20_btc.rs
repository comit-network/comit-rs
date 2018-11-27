use bitcoin_support::BitcoinQuantity;
use ethereum_support::Erc20Quantity;
use swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        actions::{
            bitcoin::{BitcoinFund, BitcoinRefund},
            ethereum::Erc20Redeem,
            Accept, Action, Decline, StateActions,
        },
        roles::Bob,
        state_machine::*,
    },
};

impl StateActions for SwapStates<Bob<Ethereum, Bitcoin, Erc20Quantity, BitcoinQuantity>> {
    type Accept = Accept;
    type Decline = Decline;
    type Deploy = ();
    type Fund = BitcoinFund;
    type Redeem = Erc20Redeem;
    type Refund = BitcoinRefund;

    fn actions(&self) -> Vec<Action<Accept, Decline, (), BitcoinFund, Erc20Redeem, BitcoinRefund>> {
        use self::SwapStates as SS;
        match *self {
            SS::Start { .. } => vec![Action::Accept(Accept), Action::Decline(Decline)],
            SS::AlphaFunded(AlphaFunded { ref swap, .. }) => vec![Action::Fund(BitcoinFund::new(
                swap.beta_htlc_params().compute_address(),
                swap.beta_asset,
            ))],
            SS::AlphaFundedBetaDeployed(AlphaFundedBetaDeployed { .. }) => vec![], // TODO: Return Beta Funding action
            SS::BothFunded(BothFunded {
                ref swap,
                ref beta_htlc_location,
                ..
            }) => vec![Action::Refund(BitcoinRefund::new(
                *beta_htlc_location,
                swap.beta_htlc_params().into(),
                swap.beta_asset,
                swap.beta_ledger_refund_identity,
            ))],
            SS::AlphaRedeemedBetaFunded(AlphaRedeemedBetaFunded {
                ref swap,
                ref beta_htlc_location,
                ..
            }) => vec![Action::Refund(BitcoinRefund::new(
                *beta_htlc_location,
                swap.beta_htlc_params().into(),
                swap.beta_asset,
                swap.beta_ledger_refund_identity,
            ))],
            SS::AlphaRefundedBetaFunded(AlphaRefundedBetaFunded {
                ref swap,
                ref beta_htlc_location,
                ..
            }) => vec![Action::Refund(BitcoinRefund::new(
                *beta_htlc_location,
                swap.beta_htlc_params().into(),
                swap.beta_asset,
                swap.beta_ledger_refund_identity,
            ))],
            SS::AlphaFundedBetaRedeemed(AlphaFundedBetaRedeemed {
                ref alpha_htlc_location,
                ref secret,
                ..
            }) => vec![Action::Redeem(Erc20Redeem::new(
                *alpha_htlc_location,
                *secret,
            ))],
            _ => vec![],
        }
    }
}
