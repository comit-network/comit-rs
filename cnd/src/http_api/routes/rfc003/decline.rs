use crate::{
    http_api::action::ListRequiredFields,
    swap_protocols::rfc003::{actions::Decline, messages::SwapDeclineReason, Ledger},
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DeclineBody {
    pub reason: SwapDeclineReason,
}

impl<AL: Ledger, BL: Ledger> ListRequiredFields for Decline<AL, BL> {
    fn list_required_fields() -> Vec<siren::Field> {
        vec![siren::Field {
            name: "reason".to_owned(),
            class: vec![],
            _type: Some("text".to_owned()),
            value: None,
            title: None,
        }]
    }
}
