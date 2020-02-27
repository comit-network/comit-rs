use crate::{
    asset::Asset,
    swap_protocols::rfc003::{ledger_state::LedgerState, Ledger},
};

pub trait ActorState: Send + 'static {
    type AL: Ledger;
    type BL: Ledger;
    type AA: Asset;
    type BA: Asset;

    fn expected_alpha_asset(&self) -> &Self::AA;
    fn expected_beta_asset(&self) -> &Self::BA;

    fn alpha_ledger_mut(
        &mut self,
    ) -> &mut LedgerState<
        <Self::AL as Ledger>::HtlcLocation,
        <Self::AL as Ledger>::Transaction,
        Self::AA,
    >;
    fn beta_ledger_mut(
        &mut self,
    ) -> &mut LedgerState<
        <Self::BL as Ledger>::HtlcLocation,
        <Self::BL as Ledger>::Transaction,
        Self::BA,
    >;

    /// Returns true if the current swap failed at some stage.
    fn swap_failed(&self) -> bool;

    /// An error during swap execution results in this being called.  We
    /// specifically do not support setting this to `false` because currently a
    /// failed swap cannot be restarted.
    fn set_swap_failed(&mut self);
}
