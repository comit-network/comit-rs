use bitcoin_support::BitcoinQuantity;
use ethereum_support::Erc20Quantity;
use swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        actions::{
            bitcoin::BitcoinRedeem,
            ethereum::{Erc20Deploy, Erc20Fund, Erc20Refund},
            Accept, Action, Decline, StateActions,
        },
        ethereum::{Erc20Htlc, Htlc},
        roles::Bob,
        state_machine::*,
    },
};

impl StateActions for SwapStates<Bob<Bitcoin, Ethereum, BitcoinQuantity, Erc20Quantity>> {
    type Accept = Accept;
    type Decline = Decline;
    type Deploy = Erc20Deploy;
    type Fund = Erc20Fund;
    type Redeem = BitcoinRedeem;
    type Refund = Erc20Refund;

    fn actions(
        &self,
    ) -> Vec<Action<Accept, Decline, Erc20Deploy, Erc20Fund, BitcoinRedeem, Erc20Refund>> {
        use self::SwapStates as SS;
        match *self {
            SS::Start { .. } => vec![Action::Accept(Accept), Action::Decline(Decline)],
            SS::AlphaFunded(AlphaFunded { ref swap, .. }) => {
                let htlc: Erc20Htlc = swap.beta_htlc_params().into();
                vec![Action::Deploy(Erc20Deploy::new(
                    htlc.compile_to_hex().into(),
                ))]
            }
            SS::AlphaFundedBetaDeployed(AlphaFundedBetaDeployed {
                ref swap,
                ref beta_htlc_location,
                ..
            }) => {
                let htlc: Erc20Htlc = swap.beta_htlc_params().into();
                vec![Action::Fund(Erc20Fund::new(
                    htlc.token_contract_address(),
                    htlc.funding_tx_payload(*beta_htlc_location),
                ))]
            }
            SS::BothFunded(BothFunded {
                ref beta_htlc_location,
                ..
            }) => vec![Action::Refund(Erc20Refund::new(*beta_htlc_location))],
            SS::AlphaFundedBetaRefunded { .. } => vec![],
            SS::AlphaRedeemedBetaFunded(AlphaRedeemedBetaFunded {
                ref beta_htlc_location,
                ..
            }) => vec![Action::Refund(Erc20Refund::new(*beta_htlc_location))],
            SS::AlphaRefundedBetaFunded(AlphaRefundedBetaFunded {
                ref beta_htlc_location,
                ..
            }) => vec![Action::Refund(Erc20Refund::new(*beta_htlc_location))],
            SS::AlphaFundedBetaRedeemed(AlphaFundedBetaRedeemed {
                ref swap,
                ref alpha_htlc_location,
                ref secret,
                ..
            }) => vec![Action::Redeem(BitcoinRedeem::new(
                *alpha_htlc_location,
                swap.alpha_htlc_params().into(),
                swap.alpha_asset,
                swap.alpha_ledger_success_identity,
                *secret,
            ))],
            _ => vec![],
        }
    }
}
