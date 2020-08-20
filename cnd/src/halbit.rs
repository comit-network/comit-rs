use crate::{
    asset, identity, ledger, state, state::Update, tracing_ext::InstrumentProtocol, LocalSwapId,
    LockProtocol, RelativeTime, Role, Side,
};
use futures::TryStreamExt;
use std::{
    collections::{hash_map::Entry, HashMap},
    sync::Arc,
};
use tokio::sync::Mutex;

pub use comit::halbit::*;

/// The lightning invoice expiry is used to tell the receiving lnd
/// until when should the payment of this invoice can be accepted.
///
/// If a payer tries to pay an expired invoice, lnd will automatically
/// reject the payment.
///
/// In the case of halbit, there are 3 expiries to take in account:
/// 1. alpha expiry: The absolute time from when ether can be refunded to
/// Alice
/// 2. cltv or beta expiry: The relative time from when Bob can go on chain
/// to get his lightning bitcoin back. This is relative to when the
/// lightning htlc are sent to the blockchain as it uses
/// OP_CHECKSEQUENCEVERIFY.
/// 3. invoice expiry: The relative time from when Alice's lnd will not
/// accept a lightning payment from Bob. This is relative to when the hold
/// invoice is added by Alice to her lnd.
///
/// In terms of security, the beta expiry should expire before alpha expiry
/// with enough margin to ensure that Bob can refund his bitcoin (by going
/// on chain) before Alice can attempt to refund her ether.
///
/// So it should go:
/// cltv/beta expiry < min time to refund bitcoin < alpha expiry
///
/// The cltv expiry is relative so it means that once the values are agreed,
/// several actions needs to happen before we can now the actual (absolute)
/// beta expiry:
/// 1. Alice adds lnd invoice
/// 2. Bob send lnd payment
/// 3. Bob force closes the used lightning channel by broadcasting the
/// lightning htlcs.
/// 4. The lightning htlcs are mined in a block.
/// Once step 4 is done, then it is possible to know when bob can actually
/// refund his bitcoin.
///
/// Which means the following actions matter to keep the swap atomic:
/// 1. Alice and Bob agree on cltv and alpha expiry
///   > Alice control
/// 2. Alice adds lnd invoice
///   > Invoice expiry
/// 3. Bob sends lightning payment
///   > Bob control
/// 4. Bob force closes lightning channel
///   > Bitcoin blockchain
/// 5. Lightning htlcs are mined
///   > cltv expiry
/// 6. Lightning htlcs are expired
///   > Bob control/Immediate
/// 7. Bob sends Bitcoin refund transactions
///   > Bitcoin blockchain
/// 8. Bob's Bitcoin refund transactions are securely confirmed
///   > Alpha expiry
/// 9. Ether htlc is expired
///
/// If we only extract the waiting periods:
/// 0 -> Alice
///     -> invoice expiry
///         -> Bob
///             -> Bitcoin
///                 -> cltv expiry
///                     -> Bitcoin
///                         -> Alpha expiry
///
/// Note that the invoice expiry here protects Bob from locking its bitcoins
/// late in process, at a time where he tried to back out, it would not have
/// time to refund before Alice can redeem and refund.
///
/// We are currently setting the smallest expiry for Ethereum<>Bitcoin
/// onchain swaps to 12 hours but we do not recommend from Bob should
/// refrain to lock their asset. The invoice expiry value should be set to
/// this recommendation (that we currently do not provide).
///
/// Do not that Bob should not lock their funds immediately after Alice has
/// locked hers either. Bob should wait long enough to ensure that Alice's
/// asset cannot be sent to a different address by the way of a chain
/// re-org. According to various sources, it seems that 12 confirmations on
/// Ethereum (3min24s) is the equivalent of the 6 Bitcoin confirmations.
///
/// So Bob should probably wait at least 3 minutes after Alice locks her
/// Ether but not so long as to risk getting close to the absolute alpha
/// expiry.
///
/// Hence, 1 hour expiry seems to be a fair bet.
pub const INVOICE_EXPIRY_SECS: RelativeTime = RelativeTime::new(3600);

/// HTLC Lightning Bitcoin atomic swap protocol.

/// Creates a new instance of the halbit protocol.
///
/// This wrapper functions allows us to reuse code within `cnd` without having
/// to give knowledge about tracing or the state hashmaps to the `comit` crate.
pub async fn new<C>(
    id: LocalSwapId,
    params: Params,
    role: Role,
    side: Side,
    states: Arc<States>,
    connector: C,
) where
    C: WaitForOpened + WaitForAccepted + WaitForSettled + WaitForCancelled,
{
    let mut events = comit::halbit::new(&connector, params)
        .instrument_protocol(id, role, side, LockProtocol::Halbit)
        .inspect_ok(|event| tracing::info!("yielded event {}", event))
        .inspect_err(|error| tracing::error!("swap failed with {:?}", error));

    while let Ok(Some(event)) = events.try_next().await {
        states.update(&id, event).await;
    }

    tracing::info!("swap finished");
}

/// Data required to create a swap that involves bitcoin on the lightning
/// network.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CreatedSwap {
    pub asset: asset::Bitcoin,
    pub identity: identity::Lightning,
    pub network: ledger::Bitcoin,
    pub cltv_expiry: u32,
}

/// Represents states that an invoice can be in.
#[derive(Debug, Clone, Copy)]
pub enum State {
    None,
    Opened(Opened),
    Accepted(Accepted),
    Settled(Settled),
    Cancelled(Cancelled),
}

#[derive(Default, Debug)]
pub struct States(Mutex<HashMap<LocalSwapId, State>>);

impl State {
    pub fn transition_to_opened(&mut self, opened: Opened) {
        match std::mem::replace(self, State::None) {
            State::None => *self = State::Opened(opened),
            other => panic!("expected state None, got {:?}", other),
        }
    }

    pub fn transition_to_accepted(&mut self, accepted: Accepted) {
        match std::mem::replace(self, State::None) {
            State::Opened(_) => *self = State::Accepted(accepted),
            other => panic!("expected state Opened, got {:?}", other),
        }
    }

    pub fn transition_to_settled(&mut self, settled: Settled) {
        match std::mem::replace(self, State::None) {
            State::Accepted(_) => *self = State::Settled(settled),
            other => panic!("expected state Accepted, got {:?}", other),
        }
    }

    pub fn transition_to_cancelled(&mut self, cancelled: Cancelled) {
        match std::mem::replace(self, State::None) {
            // Alice cancels invoice before Bob has accepted it.
            State::Opened(_) => *self = State::Cancelled(cancelled),
            // Alice cancels invoice after Bob has accepted it.
            State::Accepted(_) => *self = State::Cancelled(cancelled),
            other => panic!("expected state Opened or Accepted, got {:?}", other),
        }
    }
}

#[async_trait::async_trait]
impl state::Get<State> for States {
    async fn get(&self, key: &LocalSwapId) -> anyhow::Result<Option<State>> {
        let states = self.0.lock().await;
        let state = states.get(key).copied();

        Ok(state)
    }
}

#[async_trait::async_trait]
impl state::Update<Event> for States {
    async fn update(&self, key: &LocalSwapId, event: Event) {
        let mut states = self.0.lock().await;
        let entry = states.entry(*key);

        match (event, entry) {
            (Event::Started, Entry::Vacant(vacant)) => {
                vacant.insert(State::None);
            }
            (Event::Opened(opened), Entry::Occupied(mut state)) => {
                state.get_mut().transition_to_opened(opened)
            }
            (Event::Accepted(accepted), Entry::Occupied(mut state)) => {
                state.get_mut().transition_to_accepted(accepted)
            }
            (Event::Settled(settled), Entry::Occupied(mut state)) => {
                state.get_mut().transition_to_settled(settled)
            }
            (Event::Cancelled(cancelled), Entry::Occupied(mut state)) => {
                state.get_mut().transition_to_cancelled(cancelled)
            }
            (Event::Started, Entry::Occupied(_)) => {
                tracing::warn!(
                    "Received Started event for {} although state is already present",
                    key
                );
            }
            (_, Entry::Vacant(_)) => {
                tracing::warn!("State not found for {}", key);
            }
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Identities {
    pub redeem_identity: identity::Lightning,
    pub refund_identity: identity::Lightning,
}
