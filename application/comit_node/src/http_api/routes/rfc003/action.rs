use crate::{http_api::route_factory::swap_path, swap_protocols::SwapId};

pub fn new_action_link(id: &SwapId, action: &str) -> String {
    format!("{}/{}", swap_path(*id), action)
}

#[derive(strum_macros::EnumString, strum_macros::Display, Debug)]
#[strum(serialize_all = "snake_case")]
pub enum Action {
    Accept,
    Decline,
    Deploy,
    Fund,
    Refund,
    Redeem,
}
