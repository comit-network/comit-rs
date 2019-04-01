use crate::swap_protocols::{
	asset::Asset,
	rfc003::{ledger::Ledger, state_machine::SwapStates},
};
use futures::sync::mpsc;
use std::sync::RwLock;

pub trait SaveState<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>: Send + Sync {
	fn save(&self, state: SwapStates<AL, BL, AA, BA>);
}

impl<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> SaveState<AL, BL, AA, BA>
	for RwLock<Option<SwapStates<AL, BL, AA, BA>>>
{
	fn save(&self, state: SwapStates<AL, BL, AA, BA>) {
		let _self = &mut *self.write().unwrap();
		*_self = Some(state);
	}
}

impl<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> SaveState<AL, BL, AA, BA>
	for mpsc::UnboundedSender<SwapStates<AL, BL, AA, BA>>
{
	fn save(&self, state: SwapStates<AL, BL, AA, BA>) {
		// ignore error the subscriber is no longer interested in state updates
		let _ = self.unbounded_send(state);
	}
}
