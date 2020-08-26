//! Standardized COMIT Expiry Times
//!
//! The COMIT protocols rely on two expiry times in order for the swap to be
//! atomic. One expiry for the alpha ledger and one for the beta ledger. The
//! beta ledger expiry time must not elapse the alpha ledger expiry time.

mod config;

use self::config::Config;
use crate::timestamp::{self, Timestamp};
use num::integer;
use std::{cmp, fmt};
use time::Duration;

// TODO: Currently we ignore deploy for Erc20.

// TODO: Research how times are calculated on each chain and if we can compare
// time across chains? This knowledge is needed because we calculate the alpha
// expiry offset based on the beta expiry offset, if one cannot compare times on
// two different chains then this calculation is invalid.

/// Calculate a pair of expiries suitable for use with the herc20-hbit COMIT
/// protocol.
pub fn expiry_offsets_herc20_hbit() -> (AlphaOffset, BetaOffset) {
    let config = Config::herc20_hbit();
    expiry_offsets(&config)
}

/// Calculate a pair of expiries suitable for use with the hbit-herc20 COMIT
/// protocol.
pub fn expiry_offsets_hbit_herc20() -> (AlphaOffset, BetaOffset) {
    let config = Config::hbit_herc20();
    expiry_offsets(&config)
}

fn expiry_offsets(config: &Config) -> (AlphaOffset, BetaOffset) {
    let alice_needs = happy_path_swap_period_for_alice(config);
    let bob_needs = happy_path_swap_period_for_bob(config);

    // Alice redeems on beta ledger so needs time to act before the beta expiry.
    let beta_offset = alice_needs;

    // Alpha expiry must be at least 'safety window' time after beta expiry.
    let minimum_safe = beta_offset + config.bobs_safety_window();

    let alpha_offset = cmp::max(minimum_safe, bob_needs);

    (alpha_offset.into(), beta_offset.into())
}

// FIXME: Do consumers of this module need this function?
/// Convert expiry offsets to absolute expiry timestamps i.e., expiry measure in
/// seconds since epoch.
pub fn to_timestamps(
    start_at: Timestamp,
    alpha_offset: AlphaOffset,
    beta_offset: BetaOffset,
) -> (AlphaExpiry, BetaExpiry) {
    let alpha = start_at.add_duration(alpha_offset.into());
    let beta = start_at.add_duration(beta_offset.into());

    (alpha.into(), beta.into())
}

/// Current time as a UNIX timestamp from the perspective of the implementer.
///
/// Intended for getting the current time from the underlying blockchain.
/// The definition `current time` varies depending on the blockchain, this
/// always refers to the time used by the op codes used in the COMIT contracts.
#[async_trait::async_trait]
pub trait CurrentTime {
    async fn current_time(&self) -> Timestamp;
}

/// This struct provides the functionality 'what should I do next'.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Expiries<A, B> {
    /// Configuration values for calculating transition periods.
    config: Config,
    /// Alpha blockchain connector for getting the current time.
    alpha_connector: A,
    /// Beta blockchain connector for getting the current time.
    beta_connector: B,
    /// The alpha ledger expiry offset.
    alpha_offset: AlphaOffset,
    /// The beta ledger expiry offset.
    beta_offset: BetaOffset,
    /// Start of swap, in seconds since epoch.
    start_at: Timestamp,
    /// The alpha ledger expiry timestamp.
    alpha_expiry: AlphaExpiry,
    /// The beta ledger expiry timestamp.
    beta_expiry: BetaExpiry,
}

impl<A, B> Expiries<A, B>
where
    A: CurrentTime,
    B: CurrentTime,
{
    pub fn new_herc20_hbit(start_at: Timestamp, alpha_connector: A, beta_connector: B) -> Self {
        let config = Config::herc20_hbit();
        Expiries::new(config, start_at, alpha_connector, beta_connector)
    }

    pub fn new_hbit_herc20(start_at: Timestamp, alpha_connector: A, beta_connector: B) -> Self {
        let config = Config::hbit_herc20();
        Expiries::new(config, start_at, alpha_connector, beta_connector)
    }

    fn new(config: Config, start_at: Timestamp, alpha_connector: A, beta_connector: B) -> Self {
        let (alpha_offset, beta_offset) = expiry_offsets(&config);
        let (alpha_expiry, beta_expiry) = to_timestamps(start_at, alpha_offset, beta_offset);
        Expiries {
            config,
            alpha_connector,
            beta_connector,
            alpha_offset,
            beta_offset,
            start_at,
            alpha_expiry,
            beta_expiry,
        }
    }

    /// Returns the recommended next action that Alice should take.
    pub async fn next_action_for_alice(&self, current_state: AliceState) -> AliceAction {
        if current_state == AliceState::Done {
            return AliceAction::NoFurtherAction;
        }

        if current_state == AliceState::RedeemBetaTransactionBroadcast {
            return AliceAction::NoFurtherAction;
        }

        let funded = current_state.has_broadcast_fund_transaction();

        if self.alpha_expiry_has_elapsed().await {
            if funded {
                return AliceAction::Refund;
            }
            return AliceAction::Abort;
        }

        let both_parties_can_complete = self.alice_can_complete(current_state).await
            && self.bob_can_complete(current_state.into()).await;

        if !both_parties_can_complete {
            if funded {
                return AliceAction::WaitToRefund;
            }
            return AliceAction::Abort;
        }

        let (next_action, _state) = current_state.next();
        next_action
    }

    /// Returns the recommended next action that Bob should take.
    pub async fn next_action_for_bob(&self, current_state: BobState) -> BobAction {
        if current_state == BobState::Done {
            return BobAction::NoFurtherAction;
        }

        if current_state == BobState::RedeemAlphaTransactionBroadcast {
            return BobAction::NoFurtherAction;
        }

        // If Alice has redeemed Bob's only action is to redeem irrespective of expiry
        // time.
        if current_state == BobState::RedeemBetaTransactionSeen {
            return BobAction::RedeemAlpha;
        }

        let funded = current_state.has_broadcast_fund_transaction();

        if self.beta_expiry_has_elapsed().await {
            if funded {
                return BobAction::Refund;
            }
            return BobAction::Abort;
        };

        let both_parties_can_complete = self.alice_can_complete(current_state.into()).await
            && self.bob_can_complete(current_state).await;

        if !both_parties_can_complete {
            if funded {
                return BobAction::WaitToRefund;
            }
            return BobAction::Abort;
        }

        let (next_action, _state) = current_state.next();
        next_action
    }

    /// True if Alice has time to complete a swap (i.e. transition to done)
    /// before the beta expiry time elapses.
    pub async fn alice_can_complete(&self, current_state: AliceState) -> bool {
        let period = period_for_alice_to_complete(&self.config, current_state);
        let now = self.beta_connector.current_time().await;

        // Alice redeems on beta ledger so is concerned about the beta expiry.
        let end_time = now.add_duration(period);
        end_time < self.beta_expiry.0
    }

    /// True if Bob has time to complete a swap (i.e. transition to done)
    /// before the expiry time elapses.
    pub async fn bob_can_complete(&self, current_state: BobState) -> bool {
        let period = period_for_bob_to_complete(&self.config, current_state);
        let now = self.alpha_connector.current_time().await;

        // Bob redeems on alpha ledger so is concerned about the alpha expiry.
        let end_time = now.add_duration(period);
        end_time < self.alpha_expiry.0
    }

    /// If Alice's next action is not taken within X minutes the expiries will
    /// become un-useful. Returns X.
    pub async fn alice_should_act_within(&self, current_state: AliceState) -> Duration {
        let period = period_for_alice_to_complete(&self.config, current_state);
        let start_time = self.beta_expiry.0.sub_duration(period);
        let now = self.beta_connector.current_time().await;

        timestamp::duration_between(now, start_time)
    }

    /// If Bob's next action is not taken within X minutes the expiries will
    /// become un-useful. Returns X.
    pub async fn bob_should_act_within(&self, current_state: BobState) -> Duration {
        let period = period_for_bob_to_complete(&self.config, current_state);
        let start_time = self.alpha_expiry.0.sub_duration(period);
        let now = self.alpha_connector.current_time().await;

        timestamp::duration_between(now, start_time)
    }

    async fn alpha_expiry_has_elapsed(&self) -> bool {
        let now = self.alpha_connector.current_time().await;
        now > self.alpha_expiry.0
    }

    async fn beta_expiry_has_elapsed(&self) -> bool {
        let now = self.beta_connector.current_time().await;
        now > self.beta_expiry.0
    }
}

/// Duration for a complete happy path swap for Alice.
fn happy_path_swap_period_for_alice(config: &Config) -> Duration {
    period_for_alice_to_complete(&config, AliceState::None)
}

/// Duration for a complete happy path swap for Bob.
fn happy_path_swap_period_for_bob(config: &Config) -> Duration {
    period_for_bob_to_complete(&config, BobState::Started)
}

/// The minimum time we should allow for Alice to transition from
/// `current_state` to done.
fn period_for_alice_to_complete(config: &Config, current_state: AliceState) -> Duration {
    // Tail call recursion with an accumulator.
    fn period_to_complete(config: &Config, state: AliceState, acc: Duration) -> Duration {
        if state == AliceState::Done {
            return acc;
        }

        let (_action, next_state) = state.next();
        let transition_period = state.transition_period(config);

        period_to_complete(config, next_state, acc + transition_period)
    }

    period_to_complete(config, current_state, Duration::zero())
}

/// The minimum time we should allow for Bob to transition from
/// `current_state` to done.
fn period_for_bob_to_complete(config: &Config, current_state: BobState) -> Duration {
    // Tail call recursion with an accumulator.
    fn period_to_complete(config: &Config, state: BobState, acc: Duration) -> Duration {
        if state == BobState::Done {
            return acc;
        }

        let (_action, next_state) = state.next();
        let transition_period = state.transition_period(config);

        period_to_complete(config, next_state, acc + transition_period)
    }

    period_to_complete(config, current_state, Duration::zero())
}

impl<A, B> fmt::Display for Expiries<A, B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "config: {}\n    alpha_offset: {}\n    beta_offset: {}",
            self.config, self.alpha_offset, self.beta_offset
        )
    }
}

// Its super easy to mix these up, add types so the compiler saves us.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AlphaOffset(Duration);
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BetaOffset(Duration);
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AlphaExpiry(Timestamp);
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BetaExpiry(Timestamp);

macro_rules! impl_from {
    ($from:tt, $target:tt) => {
        impl From<$from> for $target {
            fn from(f: $from) -> Self {
                f.0
            }
        }
    };
}
impl_from!(AlphaOffset, Duration);
impl_from!(BetaOffset, Duration);
impl_from!(AlphaExpiry, Timestamp);
impl_from!(BetaExpiry, Timestamp);

macro_rules! impl_from_nested {
    ($from:tt, $target:tt) => {
        impl From<$from> for $target {
            fn from(f: $from) -> Self {
                Self(f)
            }
        }
    };
}
impl_from_nested!(Duration, AlphaOffset);
impl_from_nested!(Duration, BetaOffset);
impl_from_nested!(Timestamp, AlphaExpiry);
impl_from_nested!(Timestamp, BetaExpiry);

impl fmt::Display for AlphaOffset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let secs = self.0.whole_seconds();
        write!(f, "{}", human_readable(secs))
    }
}

impl fmt::Display for BetaOffset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let secs = self.0.whole_seconds();
        write!(f, "{}", human_readable(secs))
    }
}

fn human_readable(seconds: i64) -> String {
    const SECONDS_IN_A_MINUTE: i64 = 60;
    const SECONDS_IN_AN_HOUR: i64 = SECONDS_IN_A_MINUTE * 60;
    const SECONDS_IN_A_DAY: i64 = SECONDS_IN_AN_HOUR * 24;

    let (days, days_rem) = integer::div_rem(seconds, SECONDS_IN_A_DAY);
    let (hours, hours_rem) = integer::div_rem(days_rem, SECONDS_IN_AN_HOUR);
    let (mins, secs) = integer::div_rem(hours_rem, SECONDS_IN_A_MINUTE);

    format!(
        "({} total seconds) {} days {} hours {} minutes {} seconds",
        seconds, days, hours, mins, secs
    )
}

/// Happy path states for Alice.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AliceState {
    /// Initial state, swap not yet started.
    None,
    /// Swap has been started.
    Started,
    /// The fund alpha transaction has been broadcast to the network.
    FundAlphaTransactionBroadcast,
    /// Implies fund alpha transaction has reached finality.
    AlphaFunded,
    /// Implies fund beta transaction has reached finality.
    BetaFunded,
    /// The redeem beta transaction has been broadcast to the network.
    RedeemBetaTransactionBroadcast,
    /// Implies beta redeem transaction has reached finality.
    Done,
}

/// Possible next action for Alice.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AliceAction {
    // Happy path actions for Alice.
    Start,
    FundAlpha,
    WaitForAlphaFundTransactionFinality,
    WaitForBetaFundTransactionFinality,
    RedeemBeta,
    WaitForBetaRedeemTransactionFinality,
    NoFurtherAction, // Implies swap is done from Alice's perspective.
    // Cancel path.
    Abort,        // Implies HTLC not funded, abort but take no further action.
    WaitToRefund, // Implies alpha ledger HTLC funded but expiry not yet elapsed.
    Refund,       // Implies HTLC funded and expiry time elapsed.
}

impl AliceState {
    /// Returns Alice's initial swap state.
    pub fn initial() -> Self {
        AliceState::None
    }

    /// Gets the next action required to transition to the next state.
    fn next(&self) -> (AliceAction, AliceState) {
        use self::{AliceAction::*, AliceState::*};

        match self {
            None => (Start, Started),
            Started => (FundAlpha, FundAlphaTransactionBroadcast),
            FundAlphaTransactionBroadcast => (WaitForAlphaFundTransactionFinality, AlphaFunded),
            AlphaFunded => (WaitForBetaFundTransactionFinality, BetaFunded),
            BetaFunded => (RedeemBeta, RedeemBetaTransactionBroadcast),
            RedeemBetaTransactionBroadcast => (WaitForBetaRedeemTransactionFinality, Done),
            Done => (NoFurtherAction, Done),
        }
    }

    /// The minimum time we need to allow to transition to the next state.
    fn transition_period(&self, c: &Config) -> Duration {
        use self::AliceAction::*;

        let (next_action, _next_state) = self.next();

        match next_action {
            Start => {
                // Transition from None to Started
                c.start()
            }
            FundAlpha => {
                // Transition from Started to FundAlphaTransactionBroadcast
                c.broadcast_alpha_fund_transaction()
            }
            WaitForAlphaFundTransactionFinality => {
                // Transition from FundAlphaTransactionBroadcast to AlphaFunded
                c.mine_alpha_fund_transaction() + c.finality_alpha()
            }
            WaitForBetaFundTransactionFinality => {
                // Transition from AlphaFunded to BetaFunded
                c.broadcast_beta_fund_transaction()
                    + c.mine_beta_fund_transaction()
                    + c.finality_beta()
            }
            RedeemBeta => {
                // Transition from BetaFunded to RedeemBetaTransactionBroadcast
                c.broadcast_beta_redeem_transaction()
            }
            WaitForBetaRedeemTransactionFinality => {
                // Transition from RedeemBetaTransactionBroadcast to Done
                c.mine_beta_redeem_transaction() + c.finality_beta()
            }
            // Transitioning to any of the cancel actions take, be definition, zero time.
            NoFurtherAction | Abort | WaitToRefund | Refund => Duration::zero(),
        }
    }

    /// True if Alice has funded the alpha HTLC. We return true as soon as the
    /// fund transaction has been broadcast since any time after attempting to
    /// refund is the correct cancellation action.
    fn has_broadcast_fund_transaction(&self) -> bool {
        match self {
            AliceState::None | AliceState::Started => false,
            AliceState::FundAlphaTransactionBroadcast
            | AliceState::AlphaFunded
            | AliceState::BetaFunded
            | AliceState::RedeemBetaTransactionBroadcast
            | AliceState::Done => true,
        }
    }
}

/// Happy path states for Bob.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BobState {
    // Bob does not have a None state because start time is implicit in Bob's fund Beta action.
    /// Initial state, swap has been started.
    Started,
    /// Implies fund alpha transaction has reached finality.
    AlphaFunded,
    /// The fund beta transaction has been broadcast to the network.
    FundBetaTransactionBroadcast,
    /// Implies fund beta transaction has reached finality.
    BetaFunded,
    /// The redeem beta transaction has been seen (e.g. an unconfirmed
    /// transaction in the mempool).
    RedeemBetaTransactionSeen,
    /// The redeem alpha transaction has been broadcast to the network.
    RedeemAlphaTransactionBroadcast,
    /// Implies alpha redeem transaction has reached finality.
    Done,
}

/// Possible next action for Alice.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BobAction {
    // Happy path actions for Bob.
    WaitForAlphaFundTransactionFinality,
    FundBeta,
    WaitForBetaFundTransactionFinality,
    WaitForBetaRedeemTransactionBroadcast,
    RedeemAlpha,
    WaitForAlphaRedeemTransactionFinality,
    NoFurtherAction, // Implies swap is done from Bob's perspective.
    // Cancel path.
    Abort,        // Implies HTLC not funded, abort but take no further action.
    WaitToRefund, // Implies alpha ledger HTLC funded but expiry not yet elapsed.
    Refund,       // Implies HTLC funded and expiry time elapsed.
}

impl BobState {
    /// Returns Bob's initial swap state.
    pub fn initial() -> Self {
        BobState::Started
    }

    /// Gets the next action required to transition to the next state.
    fn next(&self) -> (BobAction, BobState) {
        use self::{BobAction::*, BobState::*};

        match self {
            Started => (WaitForAlphaFundTransactionFinality, AlphaFunded),
            AlphaFunded => (FundBeta, FundBetaTransactionBroadcast),
            FundBetaTransactionBroadcast => (WaitForBetaFundTransactionFinality, BetaFunded),
            BetaFunded => (
                WaitForBetaRedeemTransactionBroadcast,
                RedeemBetaTransactionSeen,
            ),
            RedeemBetaTransactionSeen => (RedeemAlpha, RedeemAlphaTransactionBroadcast),
            RedeemAlphaTransactionBroadcast => (WaitForAlphaRedeemTransactionFinality, Done),
            Done => (NoFurtherAction, Done),
        }
    }

    /// The minimum time we need to allow to transition to the next state.
    fn transition_period(&self, c: &Config) -> Duration {
        use self::BobAction::*;

        let (next_action, _next_state) = self.next();

        match next_action {
            WaitForAlphaFundTransactionFinality => {
                // Transition from Started to AlphaFunded
                c.start()
                    + c.broadcast_alpha_fund_transaction()
                    + c.mine_alpha_fund_transaction()
                    + c.finality_alpha()
            }
            FundBeta => {
                // Transition from AlphaFunded to FundBetaTransactionBroadcast
                c.broadcast_beta_fund_transaction()
            }
            WaitForBetaFundTransactionFinality => {
                // Transition from FundBetaTransactionBroadcast to BetaFunded
                c.mine_beta_fund_transaction() + c.finality_beta()
            }
            // We include mine_beta_redeem_transaction since Bob will not necessarily be watching
            // the network (i.e., only watching mined blocks).
            WaitForBetaRedeemTransactionBroadcast => {
                // Transition from BetaFunded to RedeemBetaTransactionSeen
                c.broadcast_beta_redeem_transaction() + c.mine_beta_redeem_transaction()
            }
            RedeemAlpha => {
                // Transition from RedeemBetaTransactionSeen to
                // RedeemAlphaTransactionBroadcast
                c.broadcast_alpha_redeem_transaction()
            }
            WaitForAlphaRedeemTransactionFinality => {
                // Transition from RedeemAlphaTransactionBroadcast to Done
                c.mine_alpha_redeem_transaction() + c.finality_alpha()
            }
            // Transitioning to any of the cancel actions take, be definition, zero time.
            NoFurtherAction | Abort | WaitToRefund | Refund => Duration::zero(),
        }
    }

    /// True if Bob has funded the beta HTLC. We return true as soon as the
    /// fund transaction has been broadcast since any time after attempting to
    /// refund is the correct cancellation action.
    fn has_broadcast_fund_transaction(&self) -> bool {
        match self {
            BobState::Started | BobState::AlphaFunded => false,
            BobState::FundBetaTransactionBroadcast
            | BobState::BetaFunded
            | BobState::RedeemBetaTransactionSeen
            | BobState::RedeemAlphaTransactionBroadcast
            | BobState::Done => true,
        }
    }
}

/// From<'role'State> converts a state object into a best guess at the
/// counterparties state. We cannot know for sure what state the counterparty is
/// in because there are state transitions which rely on local knowledge. We
/// make a conservative guess at the counterparties state using only public
/// knowledge.

impl From<AliceState> for BobState {
    fn from(state: AliceState) -> Self {
        use AliceState::*;

        match state {
            None => Self::Started,
            Started => Self::Started,
            FundAlphaTransactionBroadcast => Self::Started,
            AlphaFunded => Self::AlphaFunded,
            // We don't look for the redeem alpha transaction so `BetaFunded` is the last of Bob's
            // states we can verify.
            BetaFunded | RedeemBetaTransactionBroadcast | Done => Self::BetaFunded,
        }
    }
}

impl From<BobState> for AliceState {
    fn from(state: BobState) -> Self {
        use BobState::*;

        match state {
            Started => Self::None,
            AlphaFunded => Self::AlphaFunded,
            FundBetaTransactionBroadcast => Self::AlphaFunded,
            BetaFunded => Self::BetaFunded,
            // We don't wait for the redeem beta transaction to reach finality so
            // `RedeemBetaTransactionBroadcast` is the last of Alice's states we can verify.
            RedeemBetaTransactionSeen | RedeemAlphaTransactionBroadcast | Done => {
                Self::RedeemBetaTransactionBroadcast
            }
        }
    }
}
