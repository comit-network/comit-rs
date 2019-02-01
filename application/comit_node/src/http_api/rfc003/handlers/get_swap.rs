use crate::{
    http_api::{asset::HttpAsset, ledger::HttpLedger, problem},
    swap_protocols::{
        asset::Asset,
        ledger::{Bitcoin, Ethereum},
        metadata_store::RoleKind,
        rfc003::{self, alice, bob, state_store::StateStore, Actions, Ledger, Timestamp},
        Metadata, MetadataStore, SwapId,
    },
};
use ethereum_support::Erc20Token;
use http_api_problem::HttpApiProblem;
use serde_json::Value;
use std::sync::Arc;

type ActionName = String;

pub fn handle_get_swap<T: MetadataStore<SwapId>, S: StateStore>(
    metadata_store: &Arc<T>,
    state_store: &Arc<S>,
    id: SwapId,
) -> Result<(GetSwapResource, Vec<ActionName>), HttpApiProblem> {
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

            let swap = SwapDescription::new(state.clone().request());
            let role = format!("{}", metadata.role);

            let communication = state.swap_communication.clone().into();
            let alpha_ledger = state.alpha_ledger_state.clone().into();
            let beta_ledger = state.beta_ledger_state.clone().into();
            let error = state.error.clone();
            let outcome = SwapOutcome::new(&communication, &alpha_ledger, &beta_ledger, &error);
            let swap_state = SwapState {
                outcome,
                communication,
                alpha_ledger,
                beta_ledger,
            };

            let actions: Vec<ActionName> =
                state.actions().iter().map(|action| action.name()).collect();

            (Ok((
                GetSwapResource {
                    swap,
                    role,
                    state: swap_state,
                },
                actions,
            )))
        })
    )
}

#[derive(Debug, Serialize)]
pub struct GetSwapResource {
    swap: SwapDescription,
    role: String,
    state: SwapState,
}

#[derive(Debug, Serialize)]
pub struct SwapDescription {
    alpha_ledger: HttpLedger,
    beta_ledger: HttpLedger,
    alpha_asset: HttpAsset,
    beta_asset: HttpAsset,
}

#[derive(Debug, Serialize)]
pub struct SwapState {
    outcome: SwapOutcome,
    communication: SwapCommunication,
    alpha_ledger: LedgerState,
    beta_ledger: LedgerState,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SwapOutcome {
    InProgress,
    Swapped,
    NotSwapped,
    InternalFailure,
}

#[derive(Debug, Serialize)]
pub struct SwapCommunication {
    current_state: SwapCommunicationState,
    alpha_expiry: Timestamp,
    beta_expiry: Timestamp,
    alpha_redeem_identity: Option<Value>,
    beta_redeem_identity: Value,
    alpha_refund_identity: Value,
    beta_refund_identity: Option<Value>,
}

#[derive(Debug, Serialize, Default)]
pub struct LedgerState {
    current_state: HtlcState,
    htlc_location: Option<Value>,
    redeem_tx: Option<Value>,
    refund_tx: Option<Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SwapCommunicationState {
    Sent,
    Accepted,
    Rejected,
}

// FIXME: Remove allow when refund is implemented
#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub enum HtlcState {
    NotDeployed,
    Deployed,
    Funded,
    Redeemed,
    Refunded,
}

impl SwapDescription {
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

impl SwapOutcome {
    pub fn new(
        swap_communication: &SwapCommunication,
        alpha_ledger: &LedgerState,
        beta_ledger: &LedgerState,
        error: &Option<rfc003::Error>,
    ) -> Self {
        let swap_communication_state = &swap_communication.current_state;
        let alpha_ledger = &alpha_ledger.current_state;
        let beta_ledger = &beta_ledger.current_state;

        use self::{HtlcState::*, SwapCommunicationState::*};
        match (swap_communication_state, alpha_ledger, beta_ledger, error) {
            (Rejected, _, _, None)
            | (Accepted, Redeemed, Refunded, None)
            | (Accepted, Refunded, Redeemed, None)
            | (Accepted, Refunded, Refunded, None) => SwapOutcome::NotSwapped,
            (Accepted, Redeemed, Redeemed, None) => SwapOutcome::Swapped,
            (Sent, NotDeployed, NotDeployed, None) | (Accepted, _, _, None) => {
                SwapOutcome::InProgress
            }
            (swap_communication_state, alpha_ledger, beta_ledger, error) => {
                warn!(
                    "Internal failure with swap communication state {:?},\
                     alpha ledger state {:?}, beta ledger state {:?} and error {:?}",
                    swap_communication_state, alpha_ledger, beta_ledger, error
                );
                SwapOutcome::InternalFailure
            }
        }
    }
}

impl<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> From<alice::SwapCommunication<AL, BL, AA, BA>>
    for SwapCommunication
{
    fn from(communication: alice::SwapCommunication<AL, BL, AA, BA>) -> Self {
        use self::alice::SwapCommunication::*;
        match communication {
            Proposed { request } => Self {
                current_state: SwapCommunicationState::Sent,
                alpha_expiry: request.alpha_expiry,
                beta_expiry: request.beta_expiry,
                alpha_redeem_identity: None,
                beta_redeem_identity: json!(request.beta_ledger_redeem_identity),
                alpha_refund_identity: json!(request.alpha_ledger_refund_identity),
                beta_refund_identity: None,
            },
            Accepted { swap_accepted, .. } => Self {
                current_state: SwapCommunicationState::Accepted,
                alpha_expiry: swap_accepted.request.alpha_expiry,
                beta_expiry: swap_accepted.request.beta_expiry,
                alpha_redeem_identity: Some(json!(swap_accepted.alpha_redeem_identity)),
                beta_redeem_identity: json!(swap_accepted.request.beta_ledger_redeem_identity),
                alpha_refund_identity: json!(swap_accepted.request.alpha_ledger_refund_identity),
                beta_refund_identity: Some(json!(swap_accepted.beta_refund_identity)),
            },
            Rejected { request, .. } => Self {
                current_state: SwapCommunicationState::Rejected,
                alpha_expiry: request.alpha_expiry,
                beta_expiry: request.beta_expiry,
                alpha_redeem_identity: None,
                beta_redeem_identity: json!(request.beta_ledger_redeem_identity),
                alpha_refund_identity: json!(request.alpha_ledger_refund_identity),
                beta_refund_identity: None,
            },
        }
    }
}

impl<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> From<bob::SwapCommunication<AL, BL, AA, BA>>
    for SwapCommunication
{
    fn from(communication: bob::SwapCommunication<AL, BL, AA, BA>) -> Self {
        use self::bob::SwapCommunication::*;
        match communication {
            Proposed { request, .. } => Self {
                current_state: SwapCommunicationState::Sent,
                alpha_expiry: request.alpha_expiry,
                beta_expiry: request.beta_expiry,
                alpha_redeem_identity: None,
                beta_redeem_identity: json!(request.beta_ledger_redeem_identity),
                alpha_refund_identity: json!(request.alpha_ledger_refund_identity),
                beta_refund_identity: None,
            },
            Accepted { swap_accepted, .. } => Self {
                current_state: SwapCommunicationState::Accepted,
                alpha_expiry: swap_accepted.request.alpha_expiry,
                beta_expiry: swap_accepted.request.beta_expiry,
                alpha_redeem_identity: Some(json!(swap_accepted.alpha_redeem_identity)),
                beta_redeem_identity: json!(swap_accepted.request.beta_ledger_redeem_identity),
                alpha_refund_identity: json!(swap_accepted.request.alpha_ledger_refund_identity),
                beta_refund_identity: Some(json!(swap_accepted.beta_refund_identity)),
            },
            Rejected { request, .. } => Self {
                current_state: SwapCommunicationState::Rejected,
                alpha_expiry: request.alpha_expiry,
                beta_expiry: request.beta_expiry,
                alpha_redeem_identity: None,
                beta_redeem_identity: json!(request.beta_ledger_redeem_identity),
                alpha_refund_identity: json!(request.alpha_ledger_refund_identity),
                beta_refund_identity: None,
            },
        }
    }
}

impl Default for HtlcState {
    fn default() -> Self {
        HtlcState::NotDeployed
    }
}

impl<L: Ledger> From<rfc003::LedgerState<L>> for LedgerState {
    fn from(ledger_state: rfc003::LedgerState<L>) -> Self {
        use self::rfc003::LedgerState::*;
        match ledger_state {
            NotDeployed => Self::default(),
            Deployed { htlc_location } => Self {
                current_state: HtlcState::Deployed,
                htlc_location: Some(json!(htlc_location)),
                ..Default::default()
            },
            Funded { htlc_location } => Self {
                current_state: HtlcState::Funded,
                htlc_location: Some(json!(htlc_location)),
                ..Default::default()
            },
            Redeemed {
                htlc_location,
                redeem_transaction,
            } => Self {
                current_state: HtlcState::Redeemed,
                htlc_location: Some(json!(htlc_location)),
                redeem_tx: Some(json!(redeem_transaction)),
                ..Default::default()
            },
            Refunded {
                htlc_location,
                refund_transaction,
            } => Self {
                current_state: HtlcState::Redeemed,
                htlc_location: Some(json!(htlc_location)),
                refund_tx: Some(json!(refund_transaction)),
                ..Default::default()
            },
        }
    }
}
