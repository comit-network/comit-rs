pub mod state;

pub mod actions {
    /// Common interface across all protocols supported by COMIT
    ///
    /// This trait is intended to be implemented on an Actor's state and return
    /// the actions which are currently available in a given state.
    pub trait Actions {
        /// Different protocols have different kinds of requirements for
        /// actions. Hence they get to choose the type here.
        type ActionKind;

        fn actions(&self) -> Vec<Self::ActionKind>;
    }

    pub use comit::actions::*;
}
