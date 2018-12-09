use bitcoin_support::BitcoinQuantity;
use ethereum_support::{Bytes, Erc20Quantity, EtherQuantity};
use swap_protocols::{
    ledger::{Ethereum, Lightning},
    rfc003::{
        actions::{
            bob::{Accept, Decline},
            Action, StateActions,
        },
        ethereum::{self, Erc20Htlc},
        lightning,
        roles::Bob,
        state_machine::*,
        Secret, SecretHash,
    },
};

impl OngoingSwap<Bob<Ethereum, Lightning, Erc20Quantity, BitcoinQuantity>> {
    pub fn redeem_action(
        &self,
        beta_htlc_location: ethereum_support::Address,
        secret: Secret,
    ) -> ethereum::SendTransaction {
        let data = Bytes::from(secret.raw_secret().to_vec());
        let gas_limit = Erc20Htlc::tx_gas_limit();

        ethereum::SendTransaction {
            to: beta_htlc_location,
            data,
            gas_limit,
            value: EtherQuantity::zero(),
        }
    }
}

impl StateActions for SwapStates<Bob<Ethereum, Lightning, Erc20Quantity, BitcoinQuantity>> {
    type Accept = Accept<Ethereum, Lightning>;
    type Decline = Decline<Ethereum, Lightning>;
    type AddInvoice = ();
    type Deploy = ();
    type Fund = lightning::SendPayment;
    type Redeem = ethereum::SendTransaction;
    type Refund = ();

    fn actions(
        &self,
    ) -> Vec<
        Action<
            Self::Accept,
            Self::Decline,
            (),
            (),
            lightning::SendPayment,
            ethereum::SendTransaction,
            (),
        >,
    > {
        use self::SwapStates as SS;
        match *self {
            SS::Start(Start { ref role, .. }) => vec![
                Action::Accept(role.accept_action()),
                Action::Decline(role.decline_action()),
            ],
            SS::AlphaFunded(AlphaFunded { ref swap, .. }) => {
                let send_payment_action = lightning::SendPayment {
                    dest: swap.beta_ledger_success_identity,
                    amt: swap.beta_asset,
                    payment_hash: SecretHash::from(swap.secret.clone()),
                    final_cltv_delta: swap.beta_ledger_lock_duration,
                };
                vec![Action::Fund(send_payment_action)]
            }
            SS::AlphaFundedBetaRedeemed(AlphaFundedBetaRedeemed {
                ref swap,
                ref alpha_htlc_location,
                ref beta_redeemed_tx,
                ..
            }) => vec![Action::Redeem(
                swap.redeem_action(*alpha_htlc_location, beta_redeemed_tx.secret),
            )],
            _ => vec![],
        }
    }
}
