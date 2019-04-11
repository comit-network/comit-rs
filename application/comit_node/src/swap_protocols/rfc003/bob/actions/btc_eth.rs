use crate::swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        self, bitcoin,
        bob::{
            self,
            actions::{Accept, Decline},
            SwapCommunication,
        },
        ethereum,
        secret_source::SecretSource,
        state_machine::HtlcParams,
        Action, Actions, LedgerState,
    },
};
use bitcoin_support::{BitcoinQuantity, OutPoint};
use bitcoin_witness::PrimedInput;
use blockchain_contracts::{ethereum::rfc003::EtherHtlc, rfc003::secret::Secret};
use ethereum_support::{Bytes, EtherQuantity};
use std::sync::Arc;

type Request = rfc003::messages::Request<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>;
type Response = rfc003::messages::AcceptResponseBody<Bitcoin, Ethereum>;

fn fund_action(request: &Request, response: &Response) -> ethereum::ContractDeploy {
    HtlcParams::new_beta_params(request, response).into()
}

fn refund_action(
    request: &Request,
    beta_htlc_location: ethereum_support::Address,
) -> ethereum::SendTransaction {
    let data = Bytes::default();
    let gas_limit = EtherHtlc::tx_gas_limit();
    let network = request.beta_ledger.network;

    ethereum::SendTransaction {
        to: beta_htlc_location,
        data,
        gas_limit,
        amount: EtherQuantity::zero(),
        network,
    }
}

fn redeem_action(
    request: &Request,
    response: &Response,
    alpha_htlc_location: OutPoint,
    secret_source: &dyn SecretSource,
    secret: Secret,
) -> bitcoin::SpendOutput {
    let alpha_asset = request.alpha_asset;
    let htlc = bitcoin::Htlc::from(HtlcParams::new_alpha_params(request, response));
    let network = request.alpha_ledger.network;

    bitcoin::SpendOutput {
        output: PrimedInput::new(
            alpha_htlc_location,
            alpha_asset,
            htlc.unlock_with_secret(secret_source.secp256k1_redeem(), &secret),
        ),
        network,
    }
}

impl Actions for bob::State<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity> {
    type ActionKind = bob::ActionKind<
        Accept<Bitcoin, Ethereum>,
        Decline<Bitcoin, Ethereum>,
        (),
        ethereum::ContractDeploy,
        bitcoin::SpendOutput,
        ethereum::SendTransaction,
    >;

    fn actions(&self) -> Vec<Action<Self::ActionKind>> {
        let (request, response) = match &self.swap_communication {
            SwapCommunication::Proposed {
                pending_response, ..
            } => {
                return vec![
                    bob::ActionKind::Accept(Accept::new(
                        pending_response.sender.clone(),
                        Arc::clone(&self.secret_source),
                    ))
                    .into_action(),
                    bob::ActionKind::Decline(Decline::new(pending_response.sender.clone()))
                        .into_action(),
                ];
            }
            SwapCommunication::Accepted {
                ref request,
                ref response,
            } => (request, response),
            _ => return vec![],
        };

        let alpha_state = &self.alpha_ledger_state;
        let beta_state = &self.beta_ledger_state;

        use self::LedgerState::*;
        let mut actions = match (alpha_state, beta_state, self.secret) {
            (Funded { htlc_location, .. }, _, Some(secret)) => {
                vec![bob::ActionKind::Redeem(redeem_action(
                    &request,
                    &response,
                    *htlc_location,
                    self.secret_source.as_ref(),
                    secret,
                ))
                .into_action()]
            }
            (Funded { .. }, NotDeployed, _) => {
                vec![bob::ActionKind::Fund(fund_action(&request, &response)).into_action()]
            }
            _ => vec![],
        };

        if let Funded { htlc_location, .. } = beta_state {
            actions.push(
                bob::ActionKind::Refund(refund_action(&request, *htlc_location))
                    .into_action()
                    .with_invalid_until(request.beta_expiry),
            )
        }

        actions
    }
}
