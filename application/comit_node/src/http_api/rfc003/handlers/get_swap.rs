use crate::{
    http_api::{asset::HttpAsset, ledger::HttpLedger, problem, rfc003::handlers::Http},
    swap_protocols::{
        asset::Asset,
        ledger::{Bitcoin, Ethereum},
        metadata_store::RoleKind,
        rfc003::{self, alice, bob, state_store::StateStore, Actions, Ledger, Timestamp},
        Metadata, MetadataStore, SwapId,
    },
};
use bitcoin_support;
use ethereum_support::{self, Erc20Token};
use http_api_problem::HttpApiProblem;
use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;

type ActionName = String;

pub fn handle_get_swap<T: MetadataStore<SwapId>, S: StateStore>(
    metadata_store: &Arc<T>,
    state_store: &Arc<S>,
    id: SwapId,
) -> Result<(Value, Vec<ActionName>), HttpApiProblem> {
    let metadata = metadata_store
        .get(&id)?
        .ok_or_else(problem::swap_not_found)?;

    with_swap_types!(
        &metadata,
        (|| {
            let state = state_store
                .get::<ROLE>(id)?
                .ok_or_else(problem::state_store)?;
            trace!("Retrieved state for {}: {:?}", id, state);

            let parameters = SwapParameters::new(state.clone().request());
            let role = format!("{}", metadata.role);

            let communication = state.swap_communication.clone().into();
            let alpha_ledger = state.alpha_ledger_state.clone().into();
            let beta_ledger = state.beta_ledger_state.clone().into();
            let error = state.error.clone();
            let status = SwapStatus::new(&communication, &alpha_ledger, &beta_ledger, &error);
            let swap_state = SwapState {
                communication,
                alpha_ledger,
                beta_ledger,
            };

            let actions: Vec<ActionName> =
                state.actions().iter().map(|action| action.name()).collect();
            serde_json::to_value(GetSwapResource {
                parameters,
                status,
                role,
                state: swap_state,
            })
            .map(|swap_resource| (swap_resource, actions))
            .map_err(problem::serialize)
        })
    )
}

#[derive(Debug, Serialize)]
#[serde(bound = "Http<AL::Transaction>: Serialize, Http<BL::Transaction>: Serialize")]
pub struct GetSwapResource<AL: Ledger, BL: Ledger> {
    parameters: SwapParameters,
    role: String,
    status: SwapStatus,
    state: SwapState<AL, BL>,
}

#[derive(Debug, Serialize)]
pub struct SwapParameters {
    alpha_ledger: HttpLedger,
    beta_ledger: HttpLedger,
    alpha_asset: HttpAsset,
    beta_asset: HttpAsset,
}

#[derive(Debug, Serialize)]
#[serde(bound = "Http<AL::Transaction>: Serialize, Http<BL::Transaction>: Serialize")]
pub struct SwapState<AL: Ledger, BL: Ledger> {
    communication: SwapCommunication<AL, BL>,
    alpha_ledger: LedgerState<AL::HtlcLocation, AL::Transaction>,
    beta_ledger: LedgerState<BL::HtlcLocation, BL::Transaction>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SwapStatus {
    InProgress,
    Swapped,
    NotSwapped,
    InternalFailure,
}

#[derive(Debug, Serialize)]
#[serde(bound = "")]
pub struct SwapCommunication<AL: Ledger, BL: Ledger> {
    status: SwapCommunicationState,
    alpha_expiry: Timestamp,
    beta_expiry: Timestamp,
    alpha_redeem_identity: Option<AL::Identity>,
    beta_redeem_identity: BL::Identity,
    alpha_refund_identity: AL::Identity,
    beta_refund_identity: Option<BL::Identity>,
}

#[derive(Debug, Serialize)]
#[serde(bound = "Http<T>: Serialize, H: Serialize")]
pub struct LedgerState<H, T> {
    status: HtlcState,
    htlc_location: Option<H>,
    deploy_tx: Option<Http<T>>,
    fund_tx: Option<Http<T>>,
    redeem_tx: Option<Http<T>>,
    refund_tx: Option<Http<T>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SwapCommunicationState {
    Sent,
    Accepted,
    Rejected,
}

#[derive(Debug, Serialize)]
pub enum HtlcState {
    NotDeployed,
    Deployed,
    Funded,
    Redeemed,
    #[allow(dead_code)]
    Refunded,
}

impl SwapParameters {
    fn new<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>(
        request: rfc003::messages::Request<AL, BL, AA, BA>,
    ) -> Self {
        Self {
            alpha_ledger: request.alpha_ledger.to_http_ledger().unwrap(),
            beta_ledger: request.beta_ledger.to_http_ledger().unwrap(),
            alpha_asset: request.alpha_asset.to_http_asset().unwrap(),
            beta_asset: request.beta_asset.to_http_asset().unwrap(),
        }
    }
}

impl SwapStatus {
    pub fn new<AL: Ledger, BL: Ledger>(
        swap_communication: &SwapCommunication<AL, BL>,
        alpha_ledger: &LedgerState<AL::HtlcLocation, AL::Transaction>,
        beta_ledger: &LedgerState<BL::HtlcLocation, BL::Transaction>,
        error: &Option<rfc003::Error>,
    ) -> Self {
        let swap_communication_state = &swap_communication.status;
        let alpha_ledger = &alpha_ledger.status;
        let beta_ledger = &beta_ledger.status;

        use self::{HtlcState::*, SwapCommunicationState::*};
        match (swap_communication_state, alpha_ledger, beta_ledger, error) {
            (Rejected, _, _, None)
            | (Accepted, Redeemed, Refunded, None)
            | (Accepted, Refunded, Redeemed, None)
            | (Accepted, Refunded, Refunded, None) => SwapStatus::NotSwapped,
            (Accepted, Redeemed, Redeemed, None) => SwapStatus::Swapped,
            (Sent, NotDeployed, NotDeployed, None) | (Accepted, _, _, None) => {
                SwapStatus::InProgress
            }
            (swap_communication_state, alpha_ledger, beta_ledger, error) => {
                warn!(
                    "Internal failure with swap communication state {:?},\
                     alpha ledger state {:?}, beta ledger state {:?} and error {:?}",
                    swap_communication_state, alpha_ledger, beta_ledger, error
                );
                SwapStatus::InternalFailure
            }
        }
    }
}

impl<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> From<alice::SwapCommunication<AL, BL, AA, BA>>
    for SwapCommunication<AL, BL>
{
    fn from(communication: alice::SwapCommunication<AL, BL, AA, BA>) -> Self {
        use self::alice::SwapCommunication::*;
        match communication {
            Proposed { request } => Self {
                status: SwapCommunicationState::Sent,
                alpha_expiry: request.alpha_expiry,
                beta_expiry: request.beta_expiry,
                alpha_redeem_identity: None,
                beta_redeem_identity: request.beta_ledger_redeem_identity,
                alpha_refund_identity: request.alpha_ledger_refund_identity,
                beta_refund_identity: None,
            },
            Accepted { request, response } => Self {
                status: SwapCommunicationState::Accepted,
                alpha_expiry: request.alpha_expiry,
                beta_expiry: request.beta_expiry,
                alpha_redeem_identity: Some(response.alpha_ledger_redeem_identity),
                beta_redeem_identity: request.beta_ledger_redeem_identity,
                alpha_refund_identity: request.alpha_ledger_refund_identity,
                beta_refund_identity: Some(response.beta_ledger_refund_identity),
            },
            Rejected { request, .. } => Self {
                status: SwapCommunicationState::Rejected,
                alpha_expiry: request.alpha_expiry,
                beta_expiry: request.beta_expiry,
                alpha_redeem_identity: None,
                beta_redeem_identity: request.beta_ledger_redeem_identity,
                alpha_refund_identity: request.alpha_ledger_refund_identity,
                beta_refund_identity: None,
            },
        }
    }
}

impl<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> From<bob::SwapCommunication<AL, BL, AA, BA>>
    for SwapCommunication<AL, BL>
{
    fn from(communication: bob::SwapCommunication<AL, BL, AA, BA>) -> Self {
        use self::bob::SwapCommunication::*;
        match communication {
            Proposed { request, .. } => Self {
                status: SwapCommunicationState::Sent,
                alpha_expiry: request.alpha_expiry,
                beta_expiry: request.beta_expiry,
                alpha_redeem_identity: None,
                beta_redeem_identity: request.beta_ledger_redeem_identity,
                alpha_refund_identity: request.alpha_ledger_refund_identity,
                beta_refund_identity: None,
            },
            Accepted { request, response } => Self {
                status: SwapCommunicationState::Accepted,
                alpha_expiry: request.alpha_expiry,
                beta_expiry: request.beta_expiry,
                alpha_redeem_identity: Some(response.alpha_ledger_redeem_identity),
                beta_redeem_identity: request.beta_ledger_redeem_identity,
                alpha_refund_identity: request.alpha_ledger_refund_identity,
                beta_refund_identity: Some(response.beta_ledger_refund_identity),
            },
            Rejected { request, .. } => Self {
                status: SwapCommunicationState::Rejected,
                alpha_expiry: request.alpha_expiry,
                beta_expiry: request.beta_expiry,
                alpha_redeem_identity: None,
                beta_redeem_identity: request.beta_ledger_redeem_identity,
                alpha_refund_identity: request.alpha_ledger_refund_identity,
                beta_refund_identity: None,
            },
        }
    }
}

// Implementation needed because Ledger doesn't have a Default
impl<H, T> Default for LedgerState<H, T> {
    fn default() -> Self {
        Self {
            status: HtlcState::default(),
            htlc_location: None,
            deploy_tx: None,
            fund_tx: None,
            redeem_tx: None,
            refund_tx: None,
        }
    }
}

impl Default for HtlcState {
    fn default() -> Self {
        HtlcState::NotDeployed
    }
}

impl<L: Ledger> From<rfc003::LedgerState<L>> for LedgerState<L::HtlcLocation, L::Transaction> {
    fn from(ledger_state: rfc003::LedgerState<L>) -> Self {
        use self::rfc003::LedgerState::*;
        match ledger_state {
            NotDeployed => Self::default(),
            Deployed {
                htlc_location,
                deploy_transaction,
            } => Self {
                status: HtlcState::Deployed,
                htlc_location: Some(htlc_location),
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
                status: HtlcState::Funded,
                htlc_location: Some(htlc_location),
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
                status: HtlcState::Redeemed,
                htlc_location: Some(htlc_location),
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
                status: HtlcState::Redeemed,
                htlc_location: Some(htlc_location),
                deploy_tx: Some(Http(deploy_transaction)),
                fund_tx: Some(Http(fund_transaction)),
                refund_tx: Some(Http(refund_transaction)),
                redeem_tx: None,
            },
        }
    }
}
