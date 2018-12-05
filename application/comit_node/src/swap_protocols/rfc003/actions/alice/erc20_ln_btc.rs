use bitcoin_support::BitcoinQuantity;
use ethereum_support::{Bytes, Erc20Quantity, EtherQuantity};
use swap_protocols::{
    ledger::{Ethereum, Lightning},
    rfc003::{
        actions::{Action, StateActions},
        ethereum::{self, Erc20Htlc, Htlc},
        lightning,
        roles::Alice,
        state_machine::*,
        SecretHash,
    },
};

impl OngoingSwap<Alice<Ethereum, Lightning, Erc20Quantity, BitcoinQuantity>> {
    pub fn deploy_action(&self) -> ethereum::ContractDeploy {
        let htlc = Erc20Htlc::from(self.alpha_htlc_params());
        let data = htlc.compile_to_hex().into();
        let gas_limit = htlc.deployment_gas_limit();

        ethereum::ContractDeploy {
            data,
            value: EtherQuantity::zero(),
            gas_limit,
        }
    }

    pub fn fund_action(
        &self,
        alpha_htlc_location: ethereum_support::Address,
    ) -> ethereum::SendTransaction {
        let htlc = Erc20Htlc::from(self.alpha_htlc_params());
        let gas_limit = Erc20Htlc::fund_tx_gas_limit();

        ethereum::SendTransaction {
            to: self.alpha_asset.token_contract(),
            data: htlc.funding_tx_payload(alpha_htlc_location),
            gas_limit,
            value: EtherQuantity::zero(),
        }
    }

    pub fn refund_action(
        &self,
        alpha_htlc_location: ethereum_support::Address,
    ) -> ethereum::SendTransaction {
        let data = Bytes::default();
        let gas_limit = Erc20Htlc::tx_gas_limit();

        ethereum::SendTransaction {
            to: alpha_htlc_location,
            data,
            gas_limit,
            value: EtherQuantity::zero(),
        }
    }
}

impl StateActions for SwapStates<Alice<Ethereum, Lightning, Erc20Quantity, BitcoinQuantity>> {
    type Accept = ();
    type Decline = ();
    type LndAddInvoice = lightning::LndAddInvoice;
    type Deploy = ethereum::ContractDeploy;
    type Fund = ethereum::SendTransaction;
    type Redeem = ();
    type Refund = ethereum::SendTransaction;

    fn actions(
        &self,
    ) -> Vec<
        Action<
            (),
            (),
            lightning::LndAddInvoice,
            ethereum::ContractDeploy,
            ethereum::SendTransaction,
            (),
            ethereum::SendTransaction,
        >,
    > {
        use self::SwapStates as SS;
        match *self {
            SS::Start(Start {
                ref secret,
                ref beta_asset,
                ..
            }) => {
                let secret_hash = SecretHash::from(secret.clone());
                let add_invoice_action = lightning::LndAddInvoice {
                    r_preimage: *secret,
                    r_hash: secret_hash,
                    value: *beta_asset,
                };
                vec![Action::LndAddInvoice(add_invoice_action)]
            }
            SS::Accepted(Accepted { ref swap, .. }) => vec![Action::Deploy(swap.deploy_action())],
            SS::AlphaDeployed(AlphaDeployed {
                ref swap,
                ref alpha_htlc_location,
                ..
            }) => vec![Action::Fund(swap.fund_action(*alpha_htlc_location))],
            SS::AlphaFundedBetaRefunded(AlphaFundedBetaRefunded {
                ref alpha_htlc_location,
                ref swap,
                ..
            })
            | SS::AlphaFundedBetaRedeemed(AlphaFundedBetaRedeemed {
                ref alpha_htlc_location,
                ref swap,
                ..
            }) => vec![Action::Refund(swap.refund_action(*alpha_htlc_location))],
            _ => vec![],
        }
    }
}
