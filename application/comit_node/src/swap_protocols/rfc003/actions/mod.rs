use crate::swap_protocols::rfc003::Timestamp;

pub mod erc20;
pub mod non_erc20;

pub trait Actions {
    type ActionKind;

    fn actions(&self) -> Vec<Action<Self::ActionKind>>;
}

#[derive(Debug)]
pub struct Action<ActionKind> {
    pub invalid_until: Option<Timestamp>,
    pub inner: ActionKind,
}

impl<ActionKind> Action<ActionKind> {
    pub fn with_invalid_until(self, invalid_until: Timestamp) -> Self {
        Action {
            invalid_until: Some(invalid_until),
            ..self
        }
    }
}
