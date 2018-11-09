use bitcoin_support::BitcoinQuantity;
use ethereum_support::EtherQuantity;
use event_store::EventStore;
use failure;
use http_api::rfc003::swap::HttpApiProblemStdError;
use http_api_problem::HttpApiProblem;
use std::{str::FromStr, sync::Arc};
use swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        roles::{Bob, Role},
        state_machine::*,
        state_store::StateStore,
    },
    MetadataStore,
};
use swaps::common::SwapId;
use warp::{self, Rejection, Reply};

#[derive(Clone, Copy, Debug)]
pub enum Action {
    Accept,
    Decline,
}

#[derive(Clone, Deserialize, Debug)]
pub struct ActionBody {}

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

pub fn post<E: EventStore<SwapId>, T: MetadataStore<SwapId>, S: StateStore<SwapId>>(
    event_store: Arc<E>,
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    id: SwapId,
    action: Action,
    body: ActionBody,
) -> Result<impl Reply, Rejection> {
    use swap_protocols::{Assets, Ledgers, Metadata, Roles};

    let result: Result<(), failure::Error> =
        match metadata_store.get(&id) {
            Err(e) => {
                error!("Issue retrieve swap metadata for id {}", id);
                Err(failure::Error::from(e))
            }
            Ok(Metadata {
                source_ledger: Ledgers::Bitcoin,
                target_ledger: Ledgers::Ethereum,
                source_asset: Assets::Bitcoin,
                target_asset: Assets::Ether,
                role,
            }) => match role {
                Roles::Alice => {
                    return Err(warp::reject::custom(HttpApiProblemStdError {
                        http_api_problem: HttpApiProblem::new("incorrect-state-for-action")
                            .set_status(400)
                            .set_detail("Only Bob can accept or decline a swap"),
                    }));
                }
                Roles::Bob => update_state::<
                    S,
                    Bob<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>,
                >(state_store, &id, action),
            },
            _ => unreachable!("No other type is expected to be found in the store"),
        };

    result.map(|_| warp::reply::with_status(warp::reply(), warp::http::StatusCode::OK))
}

fn update_state<S: StateStore<SwapId>, R: Role>(
    state_store: Arc<S>,
    id: &SwapId,
    action: Action,
) -> Result<(), failure::Error> {
    let _: Result<(), failure::Error> = match state_store.get::<R>(id) {
        Err(e) => Err(failure::Error::from(e)),
        Ok(state) => match action {
            Action::Accept => unimplemented!(),
            Action::Decline => decline::<R>(state),
        },
    };
    Ok(())
}

fn decline<R: Role>(state: SwapStates<R>) -> Result<(), failure::Error> {
    match state {
        SwapStates::Start(start) => unimplemented!(),
        _ => unimplemented!(),
    }
}
