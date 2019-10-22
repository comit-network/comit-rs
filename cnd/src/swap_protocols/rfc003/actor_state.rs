use crate::swap_protocols::{
    asset::Asset,
    rfc003::{self, ledger_state::LedgerState, secret::Secret, Ledger},
};
use std::fmt::Debug;

pub trait ActorState: Debug + Clone + Send + Sync + 'static {
    type AL: Ledger;
    type BL: Ledger;
    type AA: Asset;
    type BA: Asset;

    fn set_secret(&mut self, secret: Secret);
    fn set_error(&mut self, error: rfc003::Error);
    fn alpha_ledger_mut(&mut self) -> &mut LedgerState<Self::AL>;
    fn beta_ledger_mut(&mut self) -> &mut LedgerState<Self::BL>;
}
