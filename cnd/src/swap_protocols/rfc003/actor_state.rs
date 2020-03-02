use crate::swap_protocols::rfc003::{ledger_state::LedgerState, Ledger};

pub trait ActorState: 'static {
    type AL: Ledger;
    type BL: Ledger;
    type AA;
    type BA;
    type AH;
    type BH;

    fn expected_alpha_asset(&self) -> &Self::AA;
    fn expected_beta_asset(&self) -> &Self::BA;

    fn alpha_ledger_mut(
        &mut self,
    ) -> &mut LedgerState<Self::AA, Self::AH, <Self::AL as Ledger>::Transaction>;
    fn beta_ledger_mut(
        &mut self,
    ) -> &mut LedgerState<Self::BA, Self::BH, <Self::BL as Ledger>::Transaction>;

    /// Returns true if the current swap failed at some stage.
    fn swap_failed(&self) -> bool;

    /// An error during swap execution results in this being called.  We
    /// specifically do not support setting this to `false` because currently a
    /// failed swap cannot be restarted.
    fn set_swap_failed(&mut self);
}
