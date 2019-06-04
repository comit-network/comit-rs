use crate::{
    http_api::{Http, SwapStatus},
    swap_protocols::{
        asset::Asset,
        rfc003::{self, alice, bob, Ledger, SecretHash},
        Timestamp,
    },
};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(
    bound = "Http<AL::Identity>: Serialize, Http<BL::Identity>: Serialize,\
             Http<AL::HtlcLocation>: Serialize, Http<BL::HtlcLocation>: Serialize,\
             Http<AL::Transaction>: Serialize, Http<BL::Transaction>: Serialize"
)]
pub struct SwapState<AL: Ledger, BL: Ledger> {
    pub communication: SwapCommunication<AL::Identity, BL::Identity>,
    pub alpha_ledger: LedgerState<AL::HtlcLocation, AL::Transaction>,
    pub beta_ledger: LedgerState<BL::HtlcLocation, BL::Transaction>,
}

#[derive(Debug, Serialize)]
#[serde(bound = "Http<AI>: Serialize, Http<BI>: Serialize")]
pub struct SwapCommunication<AI, BI> {
    pub status: SwapCommunicationState,
    pub alpha_expiry: Timestamp,
    pub beta_expiry: Timestamp,
    pub alpha_redeem_identity: Option<Http<AI>>,
    pub beta_redeem_identity: Http<BI>,
    pub alpha_refund_identity: Http<AI>,
    pub beta_refund_identity: Option<Http<BI>>,
    pub secret_hash: SecretHash,
}

#[derive(Debug, Serialize, derivative::Derivative)]
#[serde(bound = "Http<T>: Serialize, Http<H>: Serialize")]
// All type variables are used inside `Option`, hence we have safe defaults without any bounds.
#[derivative(Default(bound = ""))]
pub struct LedgerState<H, T> {
    pub status: rfc003::HtlcState,
    pub htlc_location: Option<Http<H>>,
    pub deploy_tx: Option<Http<T>>,
    pub fund_tx: Option<Http<T>>,
    pub redeem_tx: Option<Http<T>>,
    pub refund_tx: Option<Http<T>>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SwapCommunicationState {
    Sent,
    Accepted,
    Rejected,
}

impl<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> From<alice::SwapCommunication<AL, BL, AA, BA>>
    for SwapCommunication<AL::Identity, BL::Identity>
{
    fn from(communication: alice::SwapCommunication<AL, BL, AA, BA>) -> Self {
        use self::alice::SwapCommunication::*;
        match communication {
            Proposed { request } => Self {
                status: SwapCommunicationState::Sent,
                alpha_expiry: request.alpha_expiry,
                beta_expiry: request.beta_expiry,
                alpha_redeem_identity: None,
                beta_redeem_identity: Http(request.beta_ledger_redeem_identity),
                alpha_refund_identity: Http(request.alpha_ledger_refund_identity),
                beta_refund_identity: None,
                secret_hash: request.secret_hash,
            },
            Accepted { request, response } => Self {
                status: SwapCommunicationState::Accepted,
                alpha_expiry: request.alpha_expiry,
                beta_expiry: request.beta_expiry,
                alpha_redeem_identity: Some(Http(response.alpha_ledger_redeem_identity)),
                beta_redeem_identity: Http(request.beta_ledger_redeem_identity),
                alpha_refund_identity: Http(request.alpha_ledger_refund_identity),
                beta_refund_identity: Some(Http(response.beta_ledger_refund_identity)),
                secret_hash: request.secret_hash,
            },
            Rejected { request, .. } => Self {
                status: SwapCommunicationState::Rejected,
                alpha_expiry: request.alpha_expiry,
                beta_expiry: request.beta_expiry,
                alpha_redeem_identity: None,
                beta_redeem_identity: Http(request.beta_ledger_redeem_identity),
                alpha_refund_identity: Http(request.alpha_ledger_refund_identity),
                beta_refund_identity: None,
                secret_hash: request.secret_hash,
            },
        }
    }
}

impl<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> From<bob::SwapCommunication<AL, BL, AA, BA>>
    for SwapCommunication<AL::Identity, BL::Identity>
{
    fn from(communication: bob::SwapCommunication<AL, BL, AA, BA>) -> Self {
        use self::bob::SwapCommunication::*;
        match communication {
            Proposed { request, .. } => Self {
                status: SwapCommunicationState::Sent,
                alpha_expiry: request.alpha_expiry,
                beta_expiry: request.beta_expiry,
                alpha_redeem_identity: None,
                beta_redeem_identity: Http(request.beta_ledger_redeem_identity),
                alpha_refund_identity: Http(request.alpha_ledger_refund_identity),
                beta_refund_identity: None,
                secret_hash: request.secret_hash,
            },
            Accepted { request, response } => Self {
                status: SwapCommunicationState::Accepted,
                alpha_expiry: request.alpha_expiry,
                beta_expiry: request.beta_expiry,
                alpha_redeem_identity: Some(Http(response.alpha_ledger_redeem_identity)),
                beta_redeem_identity: Http(request.beta_ledger_redeem_identity),
                alpha_refund_identity: Http(request.alpha_ledger_refund_identity),
                beta_refund_identity: Some(Http(response.beta_ledger_refund_identity)),
                secret_hash: request.secret_hash,
            },
            Rejected { request, .. } => Self {
                status: SwapCommunicationState::Rejected,
                alpha_expiry: request.alpha_expiry,
                beta_expiry: request.beta_expiry,
                alpha_redeem_identity: None,
                beta_redeem_identity: Http(request.beta_ledger_redeem_identity),
                alpha_refund_identity: Http(request.alpha_ledger_refund_identity),
                beta_refund_identity: None,
                secret_hash: request.secret_hash,
            },
        }
    }
}

impl<L: Ledger> From<rfc003::LedgerState<L>> for LedgerState<L::HtlcLocation, L::Transaction> {
    fn from(ledger_state: rfc003::LedgerState<L>) -> Self {
        use self::rfc003::LedgerState::*;
        let status = ledger_state.clone().into();
        match ledger_state {
            NotDeployed => Self::default(),
            Deployed {
                htlc_location,
                deploy_transaction,
            } => Self {
                status,
                htlc_location: Some(Http(htlc_location)),
                deploy_tx: Some(Http(deploy_transaction)),
                fund_tx: None,
                refund_tx: None,
                redeem_tx: None,
            },
            Funded {
                htlc_location,
                deploy_transaction,
                fund_transaction,
            } => Self {
                status,
                htlc_location: Some(Http(htlc_location)),
                deploy_tx: Some(Http(deploy_transaction)),
                fund_tx: Some(Http(fund_transaction)),
                refund_tx: None,
                redeem_tx: None,
            },
            Redeemed {
                htlc_location,
                deploy_transaction,
                fund_transaction,
                redeem_transaction,
            } => Self {
                status,
                htlc_location: Some(Http(htlc_location)),
                deploy_tx: Some(Http(deploy_transaction)),
                fund_tx: Some(Http(fund_transaction)),
                redeem_tx: Some(Http(redeem_transaction)),
                refund_tx: None,
            },
            Refunded {
                htlc_location,
                deploy_transaction,
                fund_transaction,
                refund_transaction,
            } => Self {
                status,
                htlc_location: Some(Http(htlc_location)),
                deploy_tx: Some(Http(deploy_transaction)),
                fund_tx: Some(Http(fund_transaction)),
                refund_tx: Some(Http(refund_transaction)),
                redeem_tx: None,
            },
        }
    }
}

impl SwapStatus {
    pub fn new(
        swap_communication_state: SwapCommunicationState,
        alpha_ledger: rfc003::HtlcState,
        beta_ledger: rfc003::HtlcState,
        error: &Option<rfc003::Error>,
    ) -> Self {
        use self::SwapCommunicationState::*;
        use crate::swap_protocols::rfc003::HtlcState::*;
        match (swap_communication_state, alpha_ledger, beta_ledger, error) {
            (Rejected, _, _, None)
            | (Accepted, Redeemed, Refunded, None)
            | (Accepted, Refunded, Redeemed, None)
            | (Accepted, Refunded, NotDeployed, None)
            | (Accepted, Refunded, Refunded, None) => SwapStatus::NotSwapped,
            (Accepted, Redeemed, Redeemed, None) => SwapStatus::Swapped,
            (Sent, NotDeployed, NotDeployed, None) | (Accepted, _, _, None) => {
                SwapStatus::InProgress
            }
            (swap_communication_state, alpha_ledger, beta_ledger, error) => {
                log::warn!(
                    "Internal failure with swap communication state {:?},\
                     alpha ledger state {:?}, beta ledger state {:?} and error {:?}",
                    swap_communication_state,
                    alpha_ledger,
                    beta_ledger,
                    error
                );
                SwapStatus::InternalFailure
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_alice_refunded_and_bob_never_funded_should_be_not_swapped() {
        let status = SwapStatus::new(
            SwapCommunicationState::Accepted,
            rfc003::HtlcState::Refunded,
            rfc003::HtlcState::NotDeployed,
            &None,
        );

        assert_eq!(status, SwapStatus::NotSwapped)
    }

}
