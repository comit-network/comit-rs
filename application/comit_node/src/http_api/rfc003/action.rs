use frunk;
use http_api::rfc003::swap::HttpApiProblemStdError;
use http_api_problem::HttpApiProblem;
use std::{str::FromStr, sync::Arc};
use swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::{bob::PendingResponses, Ledger},
    MetadataStore,
};
use swaps::common::SwapId;
use warp::{self, Rejection, Reply};

#[derive(Clone, Copy, Debug)]
pub enum Action {
    Accept,
    Decline,
}

impl FromStr for Action {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        match s {
            "accept" => Ok(Action::Accept),
            "decline" => Ok(Action::Decline),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Deserialize, LabelledGeneric)]
struct AcceptSwapRequestHttpBody<AL: Ledger, BL: Ledger> {
    alpha_ledger_success_identity: AL::Identity,
    beta_ledger_refund_identity: BL::Identity,
    beta_ledger_lock_duration: BL::LockDuration,
}

pub fn post<T: MetadataStore<SwapId>>(
    metadata_store: Arc<T>,
    pending_responses: Arc<PendingResponses<SwapId>>,
    id: SwapId,
    action: Action,
    body: serde_json::Value,
) -> Result<impl Reply, Rejection> {
    use swap_protocols::{AssetKind, LedgerKind, Metadata, RoleKind};

    let result = match metadata_store.get(&id) {
        Ok(Metadata {
            alpha_ledger: LedgerKind::Bitcoin,
            beta_ledger: LedgerKind::Ethereum,
            alpha_asset: AssetKind::Bitcoin,
            beta_asset: AssetKind::Ether,
            role,
        }) => match role {
            RoleKind::Alice => Err(HttpApiProblem::new("incorrect-state-for-action")
                .set_status(400)
                .set_detail("Only Bob can accept or decline a swap")),
            RoleKind::Bob => {
                update_state::<Bitcoin, Ethereum>(pending_responses.as_ref(), &id, action, body)
                    .map_err(From::from)
            }
        },
        Err(e) => {
            error!("Failed to retrieve meta data for swap {}: {:?}", id, e);
            Err(HttpApiProblem::new("swap-not-found").set_status(404))
        }
        _ => {
            warn!("Swap with id {} was found but the meta-data is unknown", id);
            Err(HttpApiProblem::new("swap-not-found").set_status(404))
        }
    };

    result
        .map(|_| warp::reply())
        .map_err(HttpApiProblemStdError::from)
        .map_err(warp::reject::custom)
}

fn update_state<AL: Ledger, BL: Ledger>(
    pending_responses: &PendingResponses<SwapId>,
    id: &SwapId,
    action: Action,
    body: serde_json::Value,
) -> Result<(), HttpApiProblem> {
    pending_responses
        .take::<AL, BL>(id)
        .ok_or_else(|| HttpApiProblem::with_title_from_status(500))
        .and_then(|pending_response| match action {
            Action::Accept => serde_json::from_value::<AcceptSwapRequestHttpBody<AL, BL>>(body)
                .map_err(|e| {
                    error!(
                        "Failed to deserialize body of accept response for swap {}: {:?}",
                        id, e
                    );
                    HttpApiProblem::new("invalid-body")
                        .set_status(400)
                        .set_detail("Failed to deserialize given body.")
                })
                .and_then(|body| {
                    pending_response
                        .send(Ok(frunk::labelled_convert_from(body)))
                        .map_err(|_| {
                            error!(
                                "Failed to send pending response of swap {} through channel",
                                id
                            );
                            HttpApiProblem::with_title_from_status(500)
                        })
                }),
            Action::Decline => Err(HttpApiProblem::with_title_from_status(500)
                .set_detail("declining a swap is not yet implemented")),
        })
}
