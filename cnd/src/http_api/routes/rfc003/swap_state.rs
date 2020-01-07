#![allow(clippy::type_repetition_in_bounds)]
use crate::{
    http_api::{Http, SwapStatus},
    swap_protocols::{
        asset::Asset,
        rfc003::{self, Ledger, SecretHash},
    },
    timestamp::Timestamp,
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

#[derive(Debug, Clone, PartialEq, Copy, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SwapCommunicationState {
    Sent,
    Accepted,
    Declined,
}

impl<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> From<rfc003::SwapCommunication<AL, BL, AA, BA>>
    for SwapCommunication<AL::Identity, BL::Identity>
{
    fn from(communication: rfc003::SwapCommunication<AL, BL, AA, BA>) -> Self {
        use rfc003::SwapCommunication::*;
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
            Declined { request, .. } => Self {
                status: SwapCommunicationState::Declined,
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
            IncorrectlyFunded {
                htlc_location,
                deploy_transaction,
                fund_transaction,
            } => Self {
                status,
                htlc_location: Some(Http(htlc_location)),
                deploy_tx: Some(Http(deploy_transaction)),
                fund_tx: Some(Http(fund_transaction)),
                redeem_tx: None,
                refund_tx: None,
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

        if let Some(e) = error {
            log::debug!(target: "http-api", "derived SwapStatus is InternalFailure because: {:?}", e);
            return SwapStatus::InternalFailure;
        }

        if swap_communication_state == Declined {
            return SwapStatus::NotSwapped;
        }

        match (alpha_ledger, beta_ledger) {
            (Redeemed, Redeemed) => SwapStatus::Swapped,
            (IncorrectlyFunded, _) => SwapStatus::NotSwapped,
            (Refunded, _) | (_, Refunded) => SwapStatus::NotSwapped,
            _ => SwapStatus::InProgress,
        }
    }
}

#[cfg(test)]
impl quickcheck::Arbitrary for SwapCommunicationState {
    fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
        match g.next_u32() % 3 {
            0 => SwapCommunicationState::Declined,
            1 => SwapCommunicationState::Accepted,
            2 => SwapCommunicationState::Sent,
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        http_api::routes::rfc003::swap_state::SwapCommunicationState::*,
        swap_protocols::rfc003::ledger_state::HtlcState::*,
    };

    #[test]
    fn given_alpha_refunded_and_beta_never_funded_should_be_not_swapped() {
        assert_eq!(
            SwapStatus::new(Accepted, Refunded, NotDeployed, &None),
            SwapStatus::NotSwapped
        )
    }

    #[test]
    fn given_alpha_incorrectly_funded_and_beta_never_deployed_should_be_no_swapped() {
        assert_eq!(
            SwapStatus::new(Accepted, IncorrectlyFunded, NotDeployed, &None),
            SwapStatus::NotSwapped
        )
    }

    #[test]
    fn given_both_refund_should_not_be_swapped() {
        assert_eq!(
            SwapStatus::new(Accepted, Refunded, Refunded, &None),
            SwapStatus::NotSwapped
        )
    }

    #[test]
    fn given_declined_should_not_be_swapped() {
        assert_eq!(
            SwapStatus::new(Declined, NotDeployed, NotDeployed, &None),
            SwapStatus::NotSwapped
        )
    }

    #[test]
    fn given_both_redeem_should_be_swapped() {
        assert_eq!(
            SwapStatus::new(Accepted, Redeemed, Redeemed, &None),
            SwapStatus::Swapped
        )
    }

    #[test]
    fn given_alpha_redeemed_and_beta_refunded_should_not_be_swapped() {
        assert_eq!(
            SwapStatus::new(Accepted, Redeemed, Refunded, &None),
            SwapStatus::NotSwapped
        )
    }

    #[test]
    fn given_sent_should_be_in_progress() {
        assert_eq!(
            SwapStatus::new(Sent, NotDeployed, NotDeployed, &None),
            SwapStatus::InProgress
        )
    }

    #[test]
    fn given_error_should_be_internal_error() {
        assert_eq!(
            SwapStatus::new(
                Sent,
                NotDeployed,
                NotDeployed,
                &Some(rfc003::Error::TimerError)
            ),
            SwapStatus::InternalFailure
        )
    }

    quickcheck::quickcheck! {
        fn test(
            swap_communication_state: SwapCommunicationState,
            alpha_state: rfc003::HtlcState,
            beta_state: rfc003::HtlcState
        ) -> bool {
            SwapStatus::new(swap_communication_state, alpha_state, beta_state, &None)
                != SwapStatus::InternalFailure
        }
    }
}
