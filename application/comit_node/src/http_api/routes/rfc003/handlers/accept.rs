use crate::{
    http_api::{
        problem,
        routes::rfc003::action::{ActionName, ToSirenAction},
    },
    swap_protocols::{
        ledger::{Bitcoin, Ethereum},
        rfc003::{
            bob,
            messages::{AcceptResponseBody, IntoAcceptResponseBody},
            state_store::StateStore,
            Actions, Ledger, SecretSource,
        },
        MetadataStore, SwapId,
    },
};
use bitcoin_support;
use ethereum_support::{self, Erc20Token};
use http_api_problem::{HttpApiProblem, StatusCode as HttpStatusCode};
use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct OnlyRedeem<L: Ledger> {
    pub alpha_ledger_redeem_identity: L::Identity,
}

impl IntoAcceptResponseBody<Ethereum, Bitcoin> for OnlyRedeem<Ethereum> {
    fn into_accept_response_body(
        self,
        secret_source: &dyn SecretSource,
    ) -> AcceptResponseBody<Ethereum, Bitcoin> {
        AcceptResponseBody {
            alpha_ledger_redeem_identity: self.alpha_ledger_redeem_identity,
            beta_ledger_refund_identity: secret_source.secp256k1_refund().into(),
        }
    }
}

impl ToSirenAction for bob::Accept<Ethereum, Bitcoin> {
    fn to_siren_action(&self, name: String, href: String) -> siren::Action {
        siren::Action {
            name,
            href,
            method: Some(http::Method::POST),
            _type: Some("application/json".to_owned()),
            fields: vec![siren::Field {
                name: "alpha_ledger_redeem_identity".to_owned(),
                class: vec!["ethereum".to_owned(), "address".to_owned()],
                _type: Some("text".to_owned()),
                value: None,
                title: None,
            }],
            class: vec![],
            title: None,
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct OnlyRefund<L: Ledger> {
    pub beta_ledger_refund_identity: L::Identity,
}

impl IntoAcceptResponseBody<Bitcoin, Ethereum> for OnlyRefund<Ethereum> {
    fn into_accept_response_body(
        self,
        secret_source: &dyn SecretSource,
    ) -> AcceptResponseBody<Bitcoin, Ethereum> {
        AcceptResponseBody {
            beta_ledger_refund_identity: self.beta_ledger_refund_identity,
            alpha_ledger_redeem_identity: secret_source.secp256k1_redeem().into(),
        }
    }
}

impl ToSirenAction for bob::Accept<Bitcoin, Ethereum> {
    fn to_siren_action(&self, name: String, href: String) -> siren::Action {
        siren::Action {
            name,
            href,
            method: Some(http::Method::POST),
            _type: Some("application/json".to_owned()),
            fields: vec![siren::Field {
                name: "beta_ledger_refund_identity".to_owned(),
                class: vec!["ethereum".to_owned(), "address".to_owned()],
                _type: Some("text".to_owned()),
                value: None,
                title: None,
            }],
            class: vec![],
            title: None,
        }
    }
}

#[allow(clippy::unit_arg, clippy::let_unit_value)]
pub fn handle_accept_action<T: MetadataStore<SwapId>, S: StateStore>(
    metadata_store: &T,
    state_store: &S,
    id: SwapId,
    body: serde_json::Value,
) -> Result<(), HttpApiProblem> {
    use crate::swap_protocols::{Metadata, RoleKind};
    let metadata = metadata_store
        .get(&id)?
        .ok_or_else(problem::swap_not_found)?;

    with_swap_types_bob!(
        &metadata,
        (|| serde_json::from_value::<BobAcceptBody>(body)
            .map_err(|e| {
                log::error!(
                    "Failed to deserialize body of accept response for swap {}: {:?}",
                    id,
                    e
                );
                problem::deserialize(&e)
            })
            .and_then(|accept_body| {
                let state = state_store
                    .get::<ROLE>(id)?
                    .ok_or_else(problem::state_store)?;

                let accept_action = {
                    state
                        .actions()
                        .into_iter()
                        .find_map(move |action| match action {
                            bob::ActionKind::Accept(accept) => Some(Ok(accept)),
                            _ => None,
                        })
                        .unwrap_or_else(|| Err(problem::invalid_action(ActionName::Accept)))?
                };

                accept_action
                    .accept(accept_body)
                    .map_err(|_| problem::action_already_done(ActionName::Accept))
            }))
    )
}
