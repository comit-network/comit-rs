//! Standardized COMIT Expiry Times
//!
//! The COMIT protocols rely on two expiry times in order for the swap to be
//! atomic. One expiry for the alpha ledger and one for the beta ledger. The
//! beta ledger expiry time must not elapse the alpha ledger expiry time.

mod config;

use crate::timestamp::{self, Timestamp};
use num::integer;
use std::{cmp, fmt};
use time::Duration;

use self::config::{Config, Protocol};

const NO_SCALE: u32 = 100; // 100% scaling factor has no effect.

// TODO: Currently we ignore deploy for Erc20.

// Note on expiry types: Any `Duration` expiry is a relative expiry, any
// `Timestamp` expiry is an absolute expiry. We can use relative expiries to
// create absolute expiries when, for example, swap execution starts.

/// Current time as a UNIX timestamp from the perspective of the implementer.
///
/// Intended for getting the current time from the underlying blockchain.
/// The definition `current time` varies depending on the blockchain, this
/// always refers to the time used by the op codes used in the COMIT contracts.
pub trait CurrentTime {
    fn current_time(&self) -> Timestamp; // TODO: This will need to be async.
}

/// Data that defines and manipulates expiry times.
///
/// Language note: We use the term 'expiries' (plural of expiry) to make it
/// explicit that there are two expiry times however we refer it as singular
/// since it is the struct, or pair of expiries, that we are referring to.
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
}

/// Calculate a pair of useful expiries, 'useful' means a swap can be
/// successfully completed with these expiries.
///
/// Multiply the resulting offsets by `scale_factor` percentage to give the
/// counterparty more time to act, it is expected that `scale_factor` is greater
/// than 100 i.e., the scaling _increases_ the size of the offsets.
pub fn calculate_expiry_offsets(
    protocol: Protocol,
    scale_factor: Option<u32>,
) -> (AlphaOffset, BetaOffset) {
    let scale_factor = match scale_factor {
        Some(factor) => {
            if factor < 100 {
                tracing::warn!("Scaling factor less than 100 makes the expiries smaller, these expiries will not be useful. Using scaling factor of 100");
                NO_SCALE
            } else {
                factor
            }
        }
        None => NO_SCALE,
    };

    let config = protocol.config();

    let alice_needs = happy_path_swap_period_for_alice(&config);
    let bob_needs = happy_path_swap_period_for_bob(&config);

    // Alice redeems on beta ledger so needs time to act before the beta expiry.
    let beta_offset = alice_needs;

    // TODO: Now that we have a buffer before beta expiry for Alice to act within
    // the alpha expiry can be the same as the beta expiry with no loss of
    // atomicity. However this introduces a new concern - there is no guarantee that
    // the time is the same on both chains. We should have some buffer between the
    // expiries based on this possible disparity.

    // Bob redeems on alpha ledger so needs time to act before the alpha expiry.
    let alpha_offset = cmp::max(bob_needs, beta_offset); // Alpha expiry must not be less than beta expiry.

    let alpha_offset = scale_up_by_factor(alpha_offset, scale_factor);
    let beta_offset = scale_up_by_factor(beta_offset, scale_factor);

    (alpha_offset.into(), beta_offset.into())
}

impl<A, B> Expiries<A, B>
where
    A: CurrentTime,
    B: CurrentTime,
{
    pub fn new(protocol: Protocol, alpha: A, beta: B, scale_factor: Option<u32>) -> Expiries<A, B> {
        let config = protocol.config();
        let (alpha_offset, beta_offset) = calculate_expiry_offsets(protocol, scale_factor);

        Expiries {
            config,
            alpha_connector: alpha,
            beta_connector: beta,
            alpha_offset,
            beta_offset,
        }
    }

    /// Returns true if expiries are useful for a swap started at `start_at`.
    pub fn is_useful(&self, start_at: Timestamp) -> bool {
        if self.alpha_offset.0 < self.beta_offset.0 {
            return false;
        }

        let alice_needs = happy_path_swap_period_for_alice(&self.config);
        if alice_needs > self.beta_offset.0 {
            return false;
        }

        let bob_needs = happy_path_swap_period_for_bob(&self.config);
        if bob_needs > self.alpha_offset.0 {
            return false;
        }

        // `start_at` could be in the past so we need to check absolute expiries.
        let (alpha_expiry, beta_expiry) = self.to_absolute(start_at);

        if !self.alice_can_complete(AliceState::initial(), beta_expiry) {
            return false;
        }

        if !self.bob_can_complete(BobState::initial(), alpha_expiry) {
            return false;
        }

        true
    }

    /// Convert expiry offsets to absolute expiries.
    pub fn to_absolute(&self, start_at: Timestamp) -> (AlphaExpiry, BetaExpiry) {
        let alpha = start_at.add_duration(self.alpha_offset.into());
        let beta = start_at.add_duration(self.beta_offset.into());

        (alpha.into(), beta.into())
    }

    /// True if Alice has time to complete a swap (i.e. transition to done)
    /// before the expiry time elapses.
    fn alice_can_complete(&self, current_state: AliceState, expiry: BetaExpiry) -> bool {
        let period = period_for_alice_to_complete(&self.config, current_state);
        let now = self.beta_connector.current_time();

        // Alice redeems on beta ledger so is concerned about the beta expiry.
        let end_time = now.add_duration(period);
        end_time < expiry.0
    }

    /// True if Bob has time to complete a swap (i.e. transition to done)
    /// before the expiry time elapses.
    fn bob_can_complete(&self, current_state: BobState, expiry: AlphaExpiry) -> bool {
        let period = period_for_bob_to_complete(&self.config, current_state);
        let now = self.alpha_connector.current_time();

        // Bob redeems on alpha ledger so is concerned about the alpha expiry.
        let end_time = now.add_duration(period);
        end_time < expiry.0
    }

    /// If Alice's next action is not taken within X minutes the expiries will
    /// become un-useful. Returns X.
    pub fn alice_should_act_within(
        &self,
        current_state: AliceState,
        expiry: BetaExpiry,
    ) -> Duration {
        let period = period_for_alice_to_complete(&self.config, current_state);
        let start_time = expiry.0.sub_duration(period);
        let now = self.beta_connector.current_time();

        timestamp::duration_between(now, start_time)
    }

    /// If Bob's next action is not taken within X minutes the expiries will
    /// become un-useful. Returns X.
    pub fn bob_should_act_within(&self, current_state: BobState, expiry: AlphaExpiry) -> Duration {
        let period = period_for_bob_to_complete(&self.config, current_state);
        let start_time = expiry.0.sub_duration(period);
        let now = self.alpha_connector.current_time();

        timestamp::duration_between(now, start_time)
    }

    /// Returns the recommended next action that Alice should take.
    pub fn next_action_for_alice(
        &self,
        current_state: AliceState,
        alpha: AlphaExpiry,
        beta: BetaExpiry,
    ) -> AliceAction {
        if current_state == AliceState::Done {
            return AliceAction::NoFurtherAction;
        }

        let funded = current_state.has_sent_fund_transaction();

        if self.alpha_expiry_has_elapsed(alpha) {
            if funded {
                return AliceAction::Refund;
            }
            return AliceAction::Abort;
        }

        let both_parties_can_complete = self.alice_can_complete(current_state, beta)
            && self.bob_can_complete(current_state.into(), alpha);

        if !both_parties_can_complete {
            if funded {
                return AliceAction::WaitForRefund;
            }
            return AliceAction::Abort;
        }

        let (next_action, _state) = current_state.next();
        next_action
    }

    /// Returns the recommended next action that Bob should take.
    pub fn next_action_for_bob(
        &self,
        current_state: BobState,
        alpha: AlphaExpiry,
        beta: BetaExpiry,
    ) -> BobAction {
        if current_state == BobState::Done {
            return BobAction::NoFurtherAction;
        }

        // If Alice has redeemed Bob's only action is to redeem irrespective of expiry
        // time.
        if current_state == BobState::RedeemBetaTransactionSeen {
            return BobAction::RedeemAlpha;
        }

        let funded = current_state.has_sent_fund_transaction();

        if self.beta_expiry_has_elapsed(beta) {
            if funded {
                return BobAction::Refund;
            }
            return BobAction::Abort;
        };

        let both_parties_can_complete = self.alice_can_complete(current_state.into(), beta)
            && self.bob_can_complete(current_state, alpha);

        if !both_parties_can_complete {
            if funded {
                return BobAction::WaitForRefund;
            }
            return BobAction::Abort;
        }

        let (next_action, _state) = current_state.next();
        next_action
    }

    fn alpha_expiry_has_elapsed(&self, expiry: AlphaExpiry) -> bool {
        let now = self.alpha_connector.current_time();
        now > expiry.0
    }

    fn beta_expiry_has_elapsed(&self, expiry: BetaExpiry) -> bool {
        let now = self.beta_connector.current_time();
        now > expiry.0
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

// Scale the duration by factor / 100 i.e., scale the duration by a
// percentage. If factor is <= 100 returns original value unscaled.
fn scale_up_by_factor(orig: Duration, factor: u32) -> Duration {
    if factor <= 100 {
        return orig;
    }

    let scaled = orig.whole_seconds() * factor as i64;
    let reduced = scaled.checked_div_euclid(100);

    match reduced {
        Some(secs) => Duration::seconds(secs as i64),
        None => {
            tracing::warn!("failed to scale {} by {}", orig.whole_seconds(), factor);
            orig
        }
    }
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
    /// The fund alpha transaction has been sent to the network.
    FundAlphaTransactionSent,
    /// Implies fund alpha transaction has reached finality.
    AlphaFunded,
    /// Implies fund beta transaction has reached finality.
    BetaFunded,
    /// The redeem beta transaction has been sent to the network.
    RedeemBetaTransactionSent,
    /// Implies beta redeem transaction has reached finality.
    Done,
}

/// Possible next action for Alice.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AliceAction {
    // Happy path actions for Alice.
    Start,
    FundAlpha,
    WaitForFundTransactionFinality,
    WaitForBetaFunded,
    RedeemBeta,
    WaitForRedeemBetaTransactionFinality,
    NoFurtherAction, // Implies swap is done from Alice's perspective.
    // Cancel path.
    Abort,         // Implies HTLC not funded, abort but take no further action.
    WaitForRefund, // Implies alpha ledger HTLC funded but expiry not yet elapsed.
    Refund,        // Implies HTLC funded and expiry time elapsed.
}

impl AliceState {
    /// Returns Alice's initial swap state.
    fn initial() -> Self {
        AliceState::None
    }

    /// Gets the next action required to transition to the next state.
    fn next(&self) -> (AliceAction, AliceState) {
        use self::{AliceAction::*, AliceState::*};

        match self {
            None => (Start, Started),
            Started => (FundAlpha, FundAlphaTransactionSent),
            FundAlphaTransactionSent => (WaitForFundTransactionFinality, AlphaFunded),
            AlphaFunded => (WaitForBetaFunded, BetaFunded),
            BetaFunded => (RedeemBeta, RedeemBetaTransactionSent),
            RedeemBetaTransactionSent => (WaitForRedeemBetaTransactionFinality, Done),
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
                // Transition from Started to FundAlphaTransactionSent
                c.create_alpha_fund_transaction()
            }
            WaitForFundTransactionFinality => {
                // Transition from FundAlphaTransactionSent to AlphaFunded
                c.mine_alpha_fund_transaction() + c.finality_alpha()
            }
            WaitForBetaFunded => {
                // Transition from AlphaFunded to BetaFunded
                c.create_beta_fund_transaction()
                    + c.mine_beta_fund_transaction()
                    + c.finality_beta()
            }
            RedeemBeta => {
                // Transition from BetaFunded to RedeemBetaTransactionSent
                c.create_beta_redeem_transaction()
            }
            WaitForRedeemBetaTransactionFinality => {
                // Transition from RedeemBetaTransactionSent to Done
                c.mine_beta_redeem_transaction() + c.finality_beta()
            }
            // Transitioning to any of the cancel actions takes, be definition, zero time.
            NoFurtherAction | Abort | WaitForRefund | Refund => Duration::zero(),
        }
    }

    /// True if Alice has funded the alpha HTLC. We return true as soon as the
    /// fund transaction has been sent since any time after attempting to refund
    /// is the correct cancellation action.
    fn has_sent_fund_transaction(&self) -> bool {
        match self {
            AliceState::None | AliceState::Started => false,
            AliceState::FundAlphaTransactionSent
            | AliceState::AlphaFunded
            | AliceState::BetaFunded
            | AliceState::RedeemBetaTransactionSent
            | AliceState::Done => true,
        }
    }
}

/// Happy path states for Bob.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BobState {
    // Bob does not have a None state because swap creation time is implicit in Bob's first time
    // to act after Alice funds.
    /// Initial state, swap has been started.
    Started,
    /// Implies fund alpha transaction has reached finality.
    AlphaFunded,
    /// The fund beta transaction has been sent to the network.
    FundBetaTransactionSent,
    /// Implies fund beta transaction has reached finality.
    BetaFunded,
    /// The redeem beta transaction has been seen (e.g. an unconfirmed
    /// transaction in the mempool).
    RedeemBetaTransactionSeen,
    /// The redeem alpha transaction has been sent to the network.
    RedeemAlphaTransactionSent,
    /// Implies alpha redeem transaction has reached finality.
    Done,
}

/// Possible next action for Alice.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BobAction {
    // Happy path actions for Bob.
    WaitForAlphaFunded,
    FundBeta,
    WaitForBetaFundTransactionFinality,
    WaitForBetaRedeemTransactionToBeSeen,
    RedeemAlpha,
    WaitForAlphaRedeemTransactionFinality,
    NoFurtherAction, // Implies swap is done from Bob's perspective.
    // Cancel path.
    Abort,         // Implies HTLC not funded, abort but take no further action.
    WaitForRefund, // Implies alpha ledger HTLC funded but expiry not yet elapsed.
    Refund,        // Implies HTLC funded and expiry time elapsed.
}

impl BobState {
    /// Returns Bob's initial swap state.
    fn initial() -> Self {
        BobState::Started
    }

    /// Gets the next action required to transition to the next state.
    fn next(&self) -> (BobAction, BobState) {
        use self::{BobAction::*, BobState::*};

        match self {
            Started => (WaitForAlphaFunded, AlphaFunded),
            AlphaFunded => (FundBeta, FundBetaTransactionSent),
            FundBetaTransactionSent => (WaitForBetaFundTransactionFinality, BetaFunded),
            BetaFunded => (
                WaitForBetaRedeemTransactionToBeSeen,
                RedeemBetaTransactionSeen,
            ),
            RedeemBetaTransactionSeen => (RedeemAlpha, RedeemAlphaTransactionSent),
            RedeemAlphaTransactionSent => (WaitForAlphaRedeemTransactionFinality, Done),
            Done => (NoFurtherAction, Done),
        }
    }

    /// The minimum time we need to allow to transition to the next state.
    fn transition_period(&self, c: &Config) -> Duration {
        use self::BobAction::*;

        let (next_action, _next_state) = self.next();

        match next_action {
            WaitForAlphaFunded => {
                // Transition from Started to AlphaFunded
                c.start()
                    + c.create_alpha_fund_transaction()
                    + c.mine_alpha_fund_transaction()
                    + c.finality_alpha()
            }
            FundBeta => {
                // Transition from AlphaFunded to FundBetaTransactionSent
                c.create_beta_fund_transaction()
            }
            WaitForBetaFundTransactionFinality => {
                // Transition from FundBetaTransactionSent to BetaFunded
                c.mine_beta_fund_transaction() + c.finality_beta()
            }
            WaitForBetaRedeemTransactionToBeSeen => {
                // Transition from BetaFunded to RedeemBetaTransactionSeen
                c.create_beta_redeem_transaction() + c.mine_beta_redeem_transaction()
            }
            RedeemAlpha => {
                // Transition from RedeemBetaTransactionSeen to
                // RedeemAlphaTransactionSent
                c.create_alpha_redeem_transaction()
            }
            WaitForAlphaRedeemTransactionFinality => {
                // Transition from RedeemAlphaTransactionSent to Done
                c.mine_alpha_redeem_transaction() + c.finality_alpha()
            }
            // Transitioning to any of the cancel actions takes, be definition, zero time.
            NoFurtherAction | Abort | WaitForRefund | Refund => Duration::zero(),
        }
    }

    /// True if Bob has funded the beta HTLC. We return true as soon as the
    /// fund transaction has been sent since any time after attempting to refund
    /// is the correct cancellation action.
    fn has_sent_fund_transaction(&self) -> bool {
        match self {
            BobState::Started | BobState::AlphaFunded => false,
            BobState::FundBetaTransactionSent
            | BobState::BetaFunded
            | BobState::RedeemBetaTransactionSeen
            | BobState::RedeemAlphaTransactionSent
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
            FundAlphaTransactionSent => Self::Started,
            AlphaFunded => Self::AlphaFunded,
            // We don't look for the redeem alpha transaction so `BetaFunded` is the last of Bob's
            // states we can verify.
            BetaFunded | RedeemBetaTransactionSent | Done => Self::BetaFunded,
        }
    }
}

impl From<BobState> for AliceState {
    fn from(state: BobState) -> Self {
        use BobState::*;

        match state {
            Started => Self::None,
            AlphaFunded => Self::AlphaFunded,
            FundBetaTransactionSent => Self::AlphaFunded,
            BetaFunded => Self::BetaFunded,
            // We don't wait for the redeem beta transaction to reach finality so
            // `RedeemBetaTransactionSent` is the last of Alice's states we can verify.
            RedeemBetaTransactionSeen | RedeemAlphaTransactionSent | Done => {
                Self::RedeemBetaTransactionSent
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectral::prelude::*;
    use time::{prelude::*, Duration};

    #[derive(Clone, Copy, Debug)]
    struct MockConnector;

    impl CurrentTime for MockConnector {
        fn current_time(&self) -> Timestamp {
            Timestamp::now()
        }
    }

    fn mock_connectors() -> (MockConnector, MockConnector) {
        (MockConnector, MockConnector)
    }

    #[test]
    fn can_calculate_useful_expiries_for_all_supported_protocols() {
        let scale = None;
        let future = Timestamp::now().plus(10);
        let (alpha, beta) = mock_connectors();

        let exp = Expiries::new(Protocol::Herc20Hbit, alpha, beta, scale);
        assert!(exp.is_useful(future));

        let exp = Expiries::new(Protocol::HbitHerc20, alpha, beta, scale);
        assert!(exp.is_useful(future));
    }

    #[test]
    fn can_calculate_useful_expiries_for_all_supported_protocols_with_scale() {
        let scale = Some(120);
        let now = Timestamp::now();
        let (alpha, beta) = mock_connectors();

        let exp = Expiries::new(Protocol::Herc20Hbit, alpha, beta, scale);
        assert!(exp.is_useful(now));

        let exp = Expiries::new(Protocol::HbitHerc20, alpha, beta, scale);
        assert!(exp.is_useful(now));
    }

    #[test]
    fn scale_up_by_factor_100_or_less_does_not_scale() {
        let d = Duration::minute();

        let scaled = scale_up_by_factor(d, 100);
        assert_eq!(d, scaled);

        let scaled = scale_up_by_factor(d, 10);
        assert_eq!(d, scaled);

        let scaled = scale_up_by_factor(d, 0);
        assert_eq!(d, scaled);
    }

    #[test]
    fn scale_up_by_factor_200_works() {
        let d = Duration::minute();
        let got = scale_up_by_factor(d, 200);
        let want = 2_i32.minutes();

        assert_that!(got).is_equal_to(want)
    }

    #[test]
    fn report() {
        let scale = Some(150);
        let (alpha, beta) = mock_connectors();

        println!(
            "creating expiries with a scaling factor of: {}",
            scale.unwrap()
        );

        let exp = Expiries::new(Protocol::Herc20Hbit, alpha, beta, scale);
        println!("\n herc20-hbit swap:\n {}", exp);

        let exp = Expiries::new(Protocol::HbitHerc20, alpha, beta, scale);
        println!("\n hbit-herc20 swap:\n {}", exp);
    }
}
