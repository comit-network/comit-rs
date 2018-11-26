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
            SS::Accepted { .. } => vec![],
            SS::AlphaDeployed { .. } => vec![],
            SS::AlphaFunded(AlphaFunded { ref swap, .. }) => vec![Action::Fund(BitcoinFund {
                address: swap.beta_htlc_params().compute_address(),
                value: swap.beta_asset,
            })],
            SS::AlphaFundedBetaDeployed(AlphaFundedBetaDeployed { .. }) => vec![], // TODO: Return Beta Funding action
            SS::BothFunded(BothFunded {
                ref swap,
                ref beta_htlc_location,
                ..
            }) => vec![Action::Refund(BitcoinRefund {
                outpoint: *beta_htlc_location,
                htlc: swap.beta_htlc_params().into(),
                value: swap.beta_asset,
                transient_keypair: swap.beta_ledger_refund_identity,
            })],
            SS::AlphaFundedBetaRefunded { .. } => vec![],
            SS::AlphaRedeemedBetaFunded(AlphaRedeemedBetaFunded {
                ref swap,
                ref beta_htlc_location,
                ..
            }) => vec![Action::Refund(BitcoinRefund {
                outpoint: *beta_htlc_location,
                htlc: swap.beta_htlc_params().into(),
                value: swap.beta_asset,
                transient_keypair: swap.beta_ledger_refund_identity,
            })],
            SS::AlphaRefundedBetaFunded(AlphaRefundedBetaFunded {
                ref swap,
                ref beta_htlc_location,
                ..
            }) => vec![Action::Refund(BitcoinRefund {
                outpoint: *beta_htlc_location,
                htlc: swap.beta_htlc_params().into(),
                value: swap.beta_asset,
                transient_keypair: swap.beta_ledger_refund_identity,
            })],
            SS::AlphaFundedBetaRedeemed(AlphaFundedBetaRedeemed {
                ref alpha_htlc_location,
                ref secret,
                ..
            }) => vec![Action::Redeem(Erc20Redeem {
                to_address: *alpha_htlc_location,
                data: *secret,
                gas_limit: 42.into(), //TODO come up with correct gas limit
                gas_cost: 42.into(),  //TODO come up with correct gas cost
            })],
            SS::Error(_) => vec![],
            SS::Final(_) => vec![],
        }
    }
}
