use crate::{
    comit_client::SwapDeclineReason,
    http_api::{action::ToSirenAction, route_factory::new_action_link},
    swap_protocols::{
        rfc003::{
            actions::{ActionKind, Decline},
            Ledger,
        },
        SwapId,
    },
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DeclineBody {
    pub reason: Option<SwapDeclineReason>,
}

impl<AL: Ledger, BL: Ledger> ToSirenAction for Decline<AL, BL> {
    fn to_siren_action(&self, id: &SwapId) -> siren::Action {
        siren::Action {
            name: "decline".to_owned(),
            href: new_action_link(id, "decline"),
            method: Some(http::Method::from(ActionKind::Decline)),
            _type: Some("application/json".to_owned()),
            fields: vec![siren::Field {
                name: "reason".to_owned(),
                class: vec![],
                _type: Some("text".to_owned()),
                value: None,
                title: None,
            }],
            class: vec![],
            title: None,
        }
    }
}
