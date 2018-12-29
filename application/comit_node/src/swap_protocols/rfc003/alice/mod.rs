pub mod actions;
mod communication_events;
mod spawner;
mod swap_request;

pub use self::{
    communication_events::*,
    spawner::*,
    swap_request::{SwapRequest, SwapRequestIdentities},
};

use crate::{
    comit_client::{self, ClientFactory},
    swap_protocols::{
        asset::Asset,
        rfc003::{
            self,
            events::LedgerEvents,
            ledger::Ledger,
            role::Initiation,
            save_state::SaveState,
            state_machine::{Context, Start, Swap, SwapOutcome},
            Role, Secret,
        },
    },
};
use futures::{future, Future};
use std::{marker::PhantomData, net::SocketAddr, sync::Arc};

#[derive(Clone, Debug)]
pub struct Alice<AL, BL, AA, BA> {
    phantom_data: PhantomData<(AL, BL, AA, BA)>,
}

impl<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> Alice<AL, BL, AA, BA> {
    pub fn new_state_machine<C: comit_client::Client>(
        initiation: Initiation<Self>,
        alpha_ledger_events: Box<dyn LedgerEvents<AL, AA>>,
        beta_ledger_events: Box<dyn LedgerEvents<BL, BA>>,
        comit_client_factory: Arc<dyn ClientFactory<C>>,
        comit_node_addr: SocketAddr,
        save_state: Arc<dyn SaveState<Self>>,
    ) -> Box<dyn Future<Item = SwapOutcome<Self>, Error = rfc003::Error> + Send> {
        let start_state = Start {
            alpha_ledger: initiation.alpha_ledger,
            beta_ledger: initiation.beta_ledger,
            alpha_asset: initiation.alpha_asset,
            beta_asset: initiation.beta_asset,
            alpha_ledger_refund_identity: initiation.alpha_ledger_refund_identity,
            beta_ledger_redeem_identity: initiation.beta_ledger_redeem_identity,
            alpha_ledger_lock_duration: initiation.alpha_ledger_lock_duration,
            secret: initiation.secret,
            role: Alice::default(),
        };
        save_state.save(start_state.clone().into());
        let comit_client = match comit_client_factory.client_for(comit_node_addr) {
            Ok(comit_client) => comit_client,
            // This mess will go away with #319
            Err(e) => {
                return Box::new(future::err(rfc003::Error::Internal(format!("{:?}", e))));
            }
        };

        let context = Context {
            alpha_ledger_events,
            beta_ledger_events,
            communication_events: Box::new(AliceToBob::new(Arc::clone(&comit_client))),
            state_repo: save_state,
        };

        Box::new(Swap::start_in(start_state, context))
    }
}

impl<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> Role for Alice<AL, BL, AA, BA> {
    type AlphaLedger = AL;
    type BetaLedger = BL;
    type AlphaAsset = AA;
    type BetaAsset = BA;
    type AlphaRedeemHtlcIdentity = AL::Identity;
    type AlphaRefundHtlcIdentity = AL::HtlcIdentity;
    type BetaRedeemHtlcIdentity = BL::HtlcIdentity;
    type BetaRefundHtlcIdentity = BL::Identity;
    type Secret = Secret;
}

impl<AL, BL, AA, BA> Default for Alice<AL, BL, AA, BA> {
    fn default() -> Self {
        Self {
            phantom_data: PhantomData,
        }
    }
}
