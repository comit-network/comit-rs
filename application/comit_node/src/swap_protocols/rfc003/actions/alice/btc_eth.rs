use bitcoin_support::BitcoinQuantity;
use ethereum_support::EtherQuantity;
use swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        actions::{
            bitcoin::{BitcoinFund, BitcoinRefund},
            ethereum::EtherRedeem,
            Action, StateActions,
        },
        roles::Alice,
        state_machine::*,
    },
};

impl StateActions for SwapStates<Alice<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>> {
    type Accept = ();
    type Decline = ();
    type Fund = BitcoinFund;
    type Redeem = EtherRedeem;
    type Refund = BitcoinRefund;

    fn actions(&self) -> Vec<Action<(), (), BitcoinFund, EtherRedeem, BitcoinRefund>> {
        use self::SwapStates as SS;
        match *self {
            SS::Start { .. } => vec![],
            SS::Accepted(Accepted { ref swap, .. }) => vec![Action::Fund(BitcoinFund {
                address: swap.alpha_htlc_params().compute_address(),
                value: swap.alpha_asset,
            })],
            SS::AlphaFunded { .. } => vec![],
            SS::BothFunded(BothFunded {
                ref alpha_htlc_location,
                ref beta_htlc_location,
                ref swap,
                ..
            }) => vec![
                Action::Redeem(EtherRedeem {
                    contract_address: *beta_htlc_location,
                    data: swap.secret,
                    gas_limit: 42.into(), //TODO come up with correct gas limit
                    gas_cost: 42.into(),  //TODO come up with correct gas cost
                }),
                Action::Refund(BitcoinRefund {
                    outpoint: *alpha_htlc_location,
                    htlc: swap.alpha_htlc_params().into(),
                    value: swap.alpha_asset,
                    transient_keypair: swap.alpha_ledger_refund_identity,
                }),
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
            }) => vec![Action::Refund(BitcoinRefund {
                outpoint: *alpha_htlc_location,
                htlc: swap.alpha_htlc_params().into(),
                value: swap.alpha_asset,
                transient_keypair: swap.alpha_ledger_refund_identity,
            })],
            SS::AlphaRefundedBetaFunded(AlphaRefundedBetaFunded {
                ref beta_htlc_location,
                ref swap,
                ..
            })
            | SS::AlphaRedeemedBetaFunded(AlphaRedeemedBetaFunded {
                ref beta_htlc_location,
                ref swap,
                ..
            }) => vec![Action::Redeem(EtherRedeem {
                contract_address: *beta_htlc_location,
                data: swap.secret,
                gas_limit: 42.into(), //TODO come up with correct gas limit
                gas_cost: 42.into(),  //TODO come up with correct gas cost
            })],
            SS::Error(_) => vec![],
            SS::Final(_) => vec![],
        }
    }
}
