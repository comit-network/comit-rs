use crate::{
    http_api::{routes::rfc003::action::ToActionName, Http},
    swap_protocols::{
        asset::Asset,
        rfc003::{self, state_store::StateStore, Actions, Ledger},
        Metadata, SwapId, SwapProtocol,
    },
};
use http::StatusCode;
use http_api_problem::HttpApiProblem;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(
    bound = "Http<AL>: Serialize, Http<BL>: Serialize, Http<AA>: Serialize, Http<BA>: Serialize,\
             Http<AL::Identity>: Serialize, Http<BL::Identity>: Serialize,\
             Http<AL::HtlcLocation>: Serialize, Http<BL::HtlcLocation>: Serialize,\
             Http<AL::Transaction>: Serialize, Http<BL::Transaction>: Serialize"
)]
pub struct SwapResource<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset, S: Serialize> {
    pub role: String,
    pub protocol: Http<SwapProtocol>,
    pub status: SwapStatus,
    pub parameters: SwapParameters<AL, BL, AA, BA>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<S>,
}

#[derive(Debug, Serialize)]
#[serde(
    bound = "Http<AL>: Serialize, Http<BL>: Serialize, Http<AA>: Serialize, Http<BA>: Serialize"
)]
pub struct SwapParameters<AL, BL, AA, BA> {
    alpha_ledger: Http<AL>,
    beta_ledger: Http<BL>,
    alpha_asset: Http<AA>,
    beta_asset: Http<BA>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SwapStatus {
    InProgress,
    Swapped,
    NotSwapped,
    InternalFailure,
}

impl<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> From<rfc003::messages::Request<AL, BL, AA, BA>>
    for SwapParameters<AL, BL, AA, BA>
{
    fn from(request: rfc003::messages::Request<AL, BL, AA, BA>) -> Self {
        Self {
            alpha_ledger: Http(request.alpha_ledger),
            alpha_asset: Http(request.alpha_asset),
            beta_ledger: Http(request.beta_ledger),
            beta_asset: Http(request.beta_asset),
        }
    }
}

pub enum IncludeState {
    Yes,
    No,
}

pub fn build_rfc003_siren_entity<S: StateStore>(
    state_store: &S,
    id: SwapId,
    metadata: Metadata,
    include_state: IncludeState,
) -> Result<siren::Entity, HttpApiProblem> {
    use crate::http_api::{
        problem,
        route_factory::swap_path,
        routes::rfc003::{action::new_action_link, SwapState},
    };
    use ethereum_support::Erc20Token;

    with_swap_types!(
        &metadata,
        (|| {
            let state = state_store
                .get::<ROLE>(id)?
                .ok_or_else(problem::state_store)?;

            // TODO: Implement From<actor::State> for SwapOutcome
            let communication = state.swap_communication.clone().into();
            let alpha_ledger = state.alpha_ledger_state.clone().into();
            let beta_ledger = state.beta_ledger_state.clone().into();
            let parameters = SwapParameters::from(state.clone().request());

            // The macro takes advantage of not needing to specify whether it uses
            // alice::ActionKind::name() or bob::ActionKind::name()
            #[allow(clippy::redundant_closure)]
            let actions = state
                .actions()
                .iter()
                .map(|action| action.to_action_name())
                .collect::<Vec<_>>();
            let error = state.error;
            let status =
                SwapStatus::new::<AL, BL>(&communication, &alpha_ledger, &beta_ledger, &error);

            let swap = SwapResource {
                status,
                protocol: Http(SwapProtocol::Rfc003),
                parameters,
                role: format!("{}", metadata.role),
                state: match include_state {
                    IncludeState::Yes => Some(SwapState::<AL, BL> {
                        communication,
                        alpha_ledger,
                        beta_ledger,
                    }),
                    IncludeState::No => None,
                },
            };

            let entity = siren::Entity::default()
                .with_properties(swap)
                .map_err(|e| {
                    log::error!("failed to set properties of entity: {:?}", e);
                    HttpApiProblem::with_title_and_type_from_status(
                        StatusCode::INTERNAL_SERVER_ERROR,
                    )
                })?
                .with_link(siren::NavigationalLink::new(&["self"], swap_path(id)))
                .with_link(siren::NavigationalLink::new(
                    &["human-protocol-spec"],
                    "https://github.com/comit-network/RFCs/blob/master/RFC-003-SWAP-Basic.md",
                ));

            let entity = actions.into_iter().fold(entity, |acc, action| {
                let link = new_action_link(&id, action);
                acc.with_link(siren::NavigationalLink::new(&[action], link))
            });

            Ok(entity)
        })
    )
}
