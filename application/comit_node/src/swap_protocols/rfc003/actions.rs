pub trait Actions {
    type ActionKind;

    fn actions(&self) -> Vec<Self::ActionKind>;
}
