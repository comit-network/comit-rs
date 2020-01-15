use crate::{
    asset::Asset,
    swap_protocols::rfc003::{ledger_state::LedgerState, Ledger},
};
use std::fmt::Debug;

pub trait ActorState: Debug + Clone + Send + Sync + 'static {
    type AL: Ledger;
    type BL: Ledger;
    type AA: Asset;
    type BA: Asset;

    fn expected_alpha_asset(&self) -> Self::AA;
    fn expected_beta_asset(&self) -> Self::BA;

    fn alpha_ledger_mut(&mut self) -> &mut LedgerState<Self::AL, Self::AA>;
    fn beta_ledger_mut(&mut self) -> &mut LedgerState<Self::BL, Self::BA>;
}
