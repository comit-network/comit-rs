use crate::{
    http_api::{action::ToSirenAction, route_factory::new_action_link},
    swap_protocols::{
        ledger::{Bitcoin, Ethereum},
        rfc003::{
            actions::{Accept, ActionKind},
            messages::{AcceptResponseBody, IntoAcceptResponseBody},
            Ledger, SecretSource,
        },
        SwapId,
    },
};
use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct OnlyRedeem<L: Ledger> {
    pub alpha_ledger_redeem_identity: L::Identity,
}

impl ToSirenAction for Accept<Ethereum, Bitcoin> {
    fn to_siren_action(&self, id: &SwapId) -> siren::Action {
        siren::Action {
            name: "accept".to_owned(),
            href: new_action_link(id, "accept"),
            method: Some(http::Method::from(ActionKind::Accept)),
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

#[derive(Deserialize, Clone, Debug)]
pub struct OnlyRefund<L: Ledger> {
    pub beta_ledger_refund_identity: L::Identity,
}

impl ToSirenAction for Accept<Bitcoin, Ethereum> {
    fn to_siren_action(&self, id: &SwapId) -> siren::Action {
        siren::Action {
            name: "accept".to_owned(),
            href: new_action_link(id, "accept"),
            method: Some(http::Method::from(ActionKind::Accept)),
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
