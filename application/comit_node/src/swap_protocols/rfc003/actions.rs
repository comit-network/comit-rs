use blockchain_contracts::rfc003::timestamp::Timestamp;

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

pub trait Actions {
    type ActionKind;

    fn actions(&self) -> Vec<Action<Self::ActionKind>>;
}
