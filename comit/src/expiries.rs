//! Standardized COMIT Expiry Times
//!
//! The COMIT protocols rely on two expiry times in order for the swap to be
//! atomic. One expiry for the alpha ledger and one for the beta ledger. The
//! beta ledger expiry time must not elapse before the alpha ledger expiry time.

mod config;

use self::config::{Config, Protocol};
use crate::{
    timestamp::{self, Timestamp},
    Network,
};
use async_trait::async_trait;
use num::integer;
use std::{cmp, fmt};
use time::Duration;

// TODO: Research how times are calculated on each chain and if we can compare
// time across chains? This knowledge is needed because we calculate the alpha
// expiry offset based on the beta expiry offset, if one cannot compare times on
// two different chains then this calculation is invalid.

/// Calculate a pair of expiries suitable for use with the herc20-hbit COMIT
/// protocol.
pub fn expiry_offsets_herc20_hbit(network: Network) -> (AlphaOffset, BetaOffset) {
    let config = Config::herc20_hbit(network);
    expiry_offsets(&config)
}

/// Calculate a pair of expiries suitable for use with the hbit-herc20 COMIT
/// protocol.
pub fn expiry_offsets_hbit_herc20(network: Network) -> (AlphaOffset, BetaOffset) {
    let config = Config::hbit_herc20(network);
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
#[async_trait]
pub trait CurrentTime {
    // Timestamps may be compared, however it is not necessarily meaningful to
    // compare the returned timestamps from two different implementations of this
    // trait.
    //
    // What this means in practice is that the alpha expiry time and the beta expiry
    // time can not be compared i.e., they do not guarantee total ordering. We must
    // be careful to not compare any timestamp or duration that includes in anyway
    // the result of this method call from more than a single ledger.
    //
    // ref: https://en.wikipedia.org/wiki/Total_order
    //      https://en.wikipedia.org/wiki/Partially_ordered_set
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
    pub fn new_herc20_hbit(
        network: Network,
        start_at: Timestamp,
        alpha_connector: A,
        beta_connector: B,
    ) -> Self {
        let config = Config::herc20_hbit(network);
        Expiries::new(config, start_at, alpha_connector, beta_connector)
    }

    pub fn new_hbit_herc20(
        network: Network,
        start_at: Timestamp,
        alpha_connector: A,
        beta_connector: B,
    ) -> Self {
        let config = Config::hbit_herc20(network);
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

        let funded = current_state.has_broadcast_fund_transaction();

        // After alpha expiry has elapsed it is no longer safe for Alice to redeem but
        // she may have already broadcast the redeem transaction. This could happen if
        // the redeem transaction takes longer than expected to reach finality.
        if self.alpha_expiry_has_elapsed().await {
            if current_state.has_broadcast_redeem_transaction() {
                return AliceAction::WaitForBetaRedeemTransactionFinality;
            }

            if funded {
                return AliceAction::Refund;
            }

            return AliceAction::Abort;
        }

        // If we are asking the next action then we have started.
        let alice_can_complete = match current_state {
            AliceState::None => self.alice_can_complete(AliceState::Started).await,
            _ => self.alice_can_complete(current_state).await,
        };
        let bob_can_complete = self.bob_can_complete(current_state.into()).await;

        if !(alice_can_complete && bob_can_complete) {
            if funded {
                // We know here that expiry has not yet elapsed.
                return AliceAction::WaitToRefund;
            }
            return AliceAction::Abort;
        }

        let (next_action, _state) = self.next_action_and_state_for_alice(current_state);
        next_action
    }

    /// Returns the recommended next action that Bob should take.
    pub async fn next_action_for_bob(&self, current_state: BobState) -> BobAction {
        if current_state == BobState::Done {
            return BobAction::NoFurtherAction;
        }

        // If Alice has redeemed Bob's only action is to redeem irrespective of expiry
        // time.
        if current_state == BobState::RedeemBetaTransactionBroadcast {
            return BobAction::RedeemAlpha;
        }

        let funded = current_state.has_broadcast_fund_transaction();

        if self.beta_expiry_has_elapsed().await {
            if current_state.has_broadcast_redeem_transaction() {
                return BobAction::WaitForAlphaRedeemTransactionFinality;
            }

            if funded {
                return BobAction::Refund;
            }

            return BobAction::Abort;
        };

        // If we are asking the next action we can assume Alice has started.
        let alice_state = current_state.into();
        let alice_can_complete = match alice_state {
            AliceState::None => self.alice_can_complete(AliceState::Started).await,
            _ => self.alice_can_complete(alice_state).await,
        };
        let bob_can_complete = self.bob_can_complete(current_state).await;

        if !(alice_can_complete && bob_can_complete) {
            if funded {
                // We know here that expiry has not yet elapsed.
                return BobAction::WaitToRefund;
            }
            return BobAction::Abort;
        }

        let (next_action, _state) = self.next_action_and_state_for_bob(current_state);
        next_action
    }

    fn next_action_and_state_for_alice(
        &self,
        current_state: AliceState,
    ) -> (AliceAction, AliceState) {
        match self.protocol() {
            Protocol::Herc20Hbit => current_state.next_herc20_hbit(),
            Protocol::HbitHerc20 => current_state.next_hbit_herc20(),
        }
    }

    fn next_action_and_state_for_bob(&self, current_state: BobState) -> (BobAction, BobState) {
        match self.protocol() {
            Protocol::Herc20Hbit => current_state.next_herc20_hbit(),
            Protocol::HbitHerc20 => current_state.next_hbit_herc20(),
        }
    }

    fn protocol(&self) -> Protocol {
        self.config.protocol()
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

        let (_action, next_state) = match config.protocol() {
            Protocol::Herc20Hbit => state.next_herc20_hbit(),
            Protocol::HbitHerc20 => state.next_hbit_herc20(),
        };
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

        let (_action, next_state) = match config.protocol() {
            Protocol::Herc20Hbit => state.next_herc20_hbit(),
            Protocol::HbitHerc20 => state.next_hbit_herc20(),
        };
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AlphaOffset(Duration);
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BetaOffset(Duration);
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AlphaExpiry(Timestamp);
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BetaExpiry(Timestamp);

// Implement From<Foo> for the wrapped types to the inner type.
macro_rules! impl_from_wrapped {
    ($from:tt, $target:tt) => {
        impl From<$from> for $target {
            fn from(f: $from) -> Self {
                f.0
            }
        }
    };
}
impl_from_wrapped!(AlphaOffset, Duration);
impl_from_wrapped!(BetaOffset, Duration);
impl_from_wrapped!(AlphaExpiry, Timestamp);
impl_from_wrapped!(BetaExpiry, Timestamp);

// Implement From<Foo> for the inner types to the wrapped type.
macro_rules! impl_from_inner {
    ($from:tt, $target:tt) => {
        impl From<$from> for $target {
            fn from(f: $from) -> Self {
                Self(f)
            }
        }
    };
}
impl_from_inner!(Duration, AlphaOffset);
impl_from_inner!(Duration, BetaOffset);
impl_from_inner!(Timestamp, AlphaExpiry);
impl_from_inner!(Timestamp, BetaExpiry);

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
    /// ERC20 only. The deploy transaction has been broadcast to the network.
    DeployAlphaTransactionBroadcast,
    /// ERC20 only. Implies deploy ERC20 HTLC has been mined.
    // We only wait for a single confirmation because that is what our current btsieve
    // implementation does. This is safe since the next action is Alice to fund and we
    // wait for the fund transaction to reach finality which implies the deploy
    // transaction has also reached finality.
    AlphaDeployed,
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
    DeployAlpha,                       // ERC20 only.
    WaitForAlphaDeployTransactionMine, // ERC20 only.
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

    /// Gets the next action required to transition to the next state for a
    /// herc20-hbit swap.
    fn next_herc20_hbit(&self) -> (AliceAction, AliceState) {
        use self::{AliceAction::*, AliceState::*};

        match self {
            None => (Start, Started),
            Started => (DeployAlpha, DeployAlphaTransactionBroadcast),
            DeployAlphaTransactionBroadcast => (WaitForAlphaDeployTransactionMine, AlphaDeployed),
            AlphaDeployed => (FundAlpha, FundAlphaTransactionBroadcast),
            FundAlphaTransactionBroadcast => (WaitForAlphaFundTransactionFinality, AlphaFunded),
            AlphaFunded => (WaitForBetaFundTransactionFinality, BetaFunded),
            BetaFunded => (RedeemBeta, RedeemBetaTransactionBroadcast),
            RedeemBetaTransactionBroadcast => (WaitForBetaRedeemTransactionFinality, Done),
            Done => (NoFurtherAction, Done),
        }
    }

    /// Gets the next action required to transition to the next state for a
    /// hbit-herc20 swap.
    fn next_hbit_herc20(&self) -> (AliceAction, AliceState) {
        use self::{AliceAction::*, AliceState::*};

        match self {
            None => (Start, Started),
            Started => (FundAlpha, FundAlphaTransactionBroadcast),
            DeployAlphaTransactionBroadcast | AlphaDeployed => {
                unreachable!("hbit-herc20 no deploy for Alice")
            }
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

        let (next_action, _next_state) = match c.protocol() {
            Protocol::Herc20Hbit => self.next_herc20_hbit(),
            Protocol::HbitHerc20 => self.next_hbit_herc20(),
        };

        match next_action {
            Start => {
                // Transition from None to Started
                c.start()
            }
            DeployAlpha => {
                // Transition from Started to DeployAlphaTransactionBroadcast
                c.broadcast_alpha_deploy_transaction()
            }
            WaitForAlphaDeployTransactionMine => {
                // Transition from DeployAlphaTransactionBroadcast to AlphaDeployed
                c.mine_alpha_deploy_transaction()
            }
            FundAlpha => {
                // Transition from Started to FundAlphaTransactionBroadcast
                // or (for herc20) from AlphaDeployed to FundAlphaTransactionBroadcast
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
        use AliceState::*;

        match self {
            None | Started | DeployAlphaTransactionBroadcast | AlphaDeployed => false,
            FundAlphaTransactionBroadcast
            | AlphaFunded
            | BetaFunded
            | RedeemBetaTransactionBroadcast
            | Done => true,
        }
    }

    /// True if Alice has broadcast her redeem transaction.
    fn has_broadcast_redeem_transaction(&self) -> bool {
        use AliceState::*;

        match self {
            None
            | Started
            | DeployAlphaTransactionBroadcast
            | AlphaDeployed
            | FundAlphaTransactionBroadcast
            | AlphaFunded
            | BetaFunded => false,
            RedeemBetaTransactionBroadcast | Done => true,
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
    /// ERC20 only. The deploy transaction has been broadcast to the network.
    DeployBetaTransactionBroadcast,
    /// ERC20 only. Implies deploy ERC20 HTLC has been mined.
    BetaDeployed,
    /// The fund beta transaction has been broadcast to the network.
    FundBetaTransactionBroadcast,
    /// Implies fund beta transaction has reached finality.
    BetaFunded,
    /// The redeem beta transaction has been broadcast to the network.
    RedeemBetaTransactionBroadcast,
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
    DeployBeta,                       // ERC20 only.
    WaitForBetaDeployTransactionMine, // ERC20 only.
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

    /// Gets the next action required to transition to the next state for a
    /// herc20-hbit swap.
    fn next_herc20_hbit(&self) -> (BobAction, BobState) {
        use self::{BobAction::*, BobState::*};

        match self {
            Started => (WaitForAlphaFundTransactionFinality, AlphaFunded),
            AlphaFunded => (FundBeta, FundBetaTransactionBroadcast),
            DeployBetaTransactionBroadcast | BetaDeployed => {
                unreachable!("herc20-hbit no deploy for Bob")
            }
            FundBetaTransactionBroadcast => (WaitForBetaFundTransactionFinality, BetaFunded),
            BetaFunded => (
                WaitForBetaRedeemTransactionBroadcast,
                RedeemBetaTransactionBroadcast,
            ),
            RedeemBetaTransactionBroadcast => (RedeemAlpha, RedeemAlphaTransactionBroadcast),
            RedeemAlphaTransactionBroadcast => (WaitForAlphaRedeemTransactionFinality, Done),
            Done => (NoFurtherAction, Done),
        }
    }

    /// Gets the next action required to transition to the next state for a
    /// hbit-herc20 swap.
    fn next_hbit_herc20(&self) -> (BobAction, BobState) {
        use self::{BobAction::*, BobState::*};

        match self {
            Started => (WaitForAlphaFundTransactionFinality, AlphaFunded),
            AlphaFunded => (DeployBeta, DeployBetaTransactionBroadcast),
            DeployBetaTransactionBroadcast => (WaitForBetaDeployTransactionMine, BetaDeployed),
            BetaDeployed => (FundBeta, FundBetaTransactionBroadcast),
            FundBetaTransactionBroadcast => (WaitForBetaFundTransactionFinality, BetaFunded),
            BetaFunded => (
                WaitForBetaRedeemTransactionBroadcast,
                RedeemBetaTransactionBroadcast,
            ),
            RedeemBetaTransactionBroadcast => (RedeemAlpha, RedeemAlphaTransactionBroadcast),
            RedeemAlphaTransactionBroadcast => (WaitForAlphaRedeemTransactionFinality, Done),
            Done => (NoFurtherAction, Done),
        }
    }

    /// The minimum time we need to allow to transition to the next state.
    fn transition_period(&self, c: &Config) -> Duration {
        use self::BobAction::*;

        let (next_action, _next_state) = match c.protocol() {
            Protocol::Herc20Hbit => self.next_herc20_hbit(),
            Protocol::HbitHerc20 => self.next_hbit_herc20(),
        };

        match next_action {
            // Once Alice starts we need at least this much time, note that c.start() is not
            // included otherwise the actual time Alice takes invalidates the swap for Bob.
            WaitForAlphaFundTransactionFinality => {
                // Transition from Started to AlphaFunded
                c.broadcast_alpha_fund_transaction()
                    + c.mine_alpha_fund_transaction()
                    + c.finality_alpha()
            }
            DeployBeta => {
                // Transition from AlphaFunded to DeployBetaTransactionBroadcast
                c.broadcast_beta_deploy_transaction()
            }
            WaitForBetaDeployTransactionMine => {
                // Transition from DeployBetaTransactionBroadcast to BetaDeployed
                c.mine_beta_deploy_transaction()
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
                // Transition from BetaFunded to RedeemBetaTransactionBroadcast
                c.broadcast_beta_redeem_transaction() + c.mine_beta_redeem_transaction()
            }
            RedeemAlpha => {
                // Transition from RedeemBetaTransactionBroadcast to
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
        use BobState::*;

        match self {
            Started | AlphaFunded | DeployBetaTransactionBroadcast | BetaDeployed => false,
            FundBetaTransactionBroadcast
            | BetaFunded
            | RedeemBetaTransactionBroadcast
            | RedeemAlphaTransactionBroadcast
            | Done => true,
        }
    }

    /// True if Bob has broadcast his redeem transaction.
    fn has_broadcast_redeem_transaction(&self) -> bool {
        use BobState::*;

        match self {
            Started
            | AlphaFunded
            | DeployBetaTransactionBroadcast
            | BetaDeployed
            | FundBetaTransactionBroadcast
            | BetaFunded
            | RedeemBetaTransactionBroadcast => false,
            RedeemAlphaTransactionBroadcast | Done => true,
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
            None
            | Started
            | DeployAlphaTransactionBroadcast
            | AlphaDeployed
            | FundAlphaTransactionBroadcast => Self::Started,
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
            AlphaFunded
            | DeployBetaTransactionBroadcast
            | BetaDeployed
            | FundBetaTransactionBroadcast => Self::AlphaFunded,
            BetaFunded => Self::BetaFunded,
            // We don't wait for the redeem beta transaction to reach finality so
            // `RedeemBetaTransactionBroadcast` is the last of Alice's states we can verify.
            RedeemBetaTransactionBroadcast | RedeemAlphaTransactionBroadcast | Done => {
                Self::RedeemBetaTransactionBroadcast
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectral::prelude::*;
    use std::sync::Arc;
    use time::{prelude::*, Duration};
    use tokio::sync::Mutex;

    #[derive(Clone, Debug)]
    struct MockConnector {
        current_time: Arc<Mutex<Timestamp>>,
    }

    #[async_trait]
    impl CurrentTime for MockConnector {
        async fn current_time(&self) -> Timestamp {
            let guard = self.current_time.lock().await;
            *guard
        }
    }

    impl Default for MockConnector {
        fn default() -> Self {
            MockConnector {
                current_time: Arc::new(Mutex::new(Timestamp::now())),
            }
        }
    }

    impl MockConnector {
        /// Create a new connector with current time incremented `inc` seconds.
        async fn inc(&self, secs: u32) {
            let mut guard = self.current_time.lock().await;
            *guard = guard.plus(secs);
        }
    }

    fn mock_connectors() -> (MockConnector, MockConnector) {
        (MockConnector::default(), MockConnector::default())
    }

    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    async fn inc_connectors(d: Duration, a: MockConnector, b: MockConnector) {
        let secs = d.whole_seconds() as u32;
        a.inc(secs).await;
        b.inc(secs).await;
    }

    // Run this test with --nocapture to see what offsets we are calculating.
    #[test]
    fn can_create_offsets() {
        fn print(p: &str, a: AlphaOffset, b: BetaOffset) {
            println!("{} expiry offsets: \n alpha: {} \n beta: {}", p, a, b);
        }

        let (a, b) = expiry_offsets_herc20_hbit(Network::Main);
        print("herc20-hbit", a, b);

        let (a, b) = expiry_offsets_hbit_herc20(Network::Main);
        print("hbit-herc20", a, b);
    }

    #[test]
    fn dev_net_herc20_hbit_expiries() {
        let (a, b) = expiry_offsets_herc20_hbit(Network::Dev);

        assert_eq!(a, 44.seconds().into());
        assert_eq!(b, 35.seconds().into());
    }

    #[tokio::test]
    async fn alice_can_complete_an_hbit_herc20_swap() {
        let start_at = Timestamp::now();
        let (ac, bc) = mock_connectors();

        let exp = Expiries::new_hbit_herc20(Network::Main, start_at, ac.clone(), bc.clone());
        let mut cur = AliceState::initial();

        let inc = 1.minutes();

        while cur != AliceState::Done {
            inc_connectors(inc, ac.clone(), bc.clone()).await;
            let (want_action, state) = cur.next_hbit_herc20();
            let got_action = exp.next_action_for_alice(cur).await;

            assert_that!(got_action).is_equal_to(want_action);

            cur = state;
        }
    }

    #[tokio::test]
    async fn alice_can_complete_an_herc20_hbit_swap() {
        let start_at = Timestamp::now();
        let (ac, bc) = mock_connectors();

        let exp = Expiries::new_herc20_hbit(Network::Main, start_at, ac.clone(), bc.clone());
        let mut cur = AliceState::initial();

        let inc = 1.minutes();

        while cur != AliceState::Done {
            inc_connectors(inc, ac.clone(), bc.clone()).await;
            let (want_action, state) = cur.next_herc20_hbit();
            let got_action = exp.next_action_for_alice(cur).await;

            assert_that!(got_action).is_equal_to(want_action);

            cur = state;
        }
    }

    #[tokio::test]
    async fn bob_can_complete_an_herc20_hbit_swap() {
        let start_at = Timestamp::now();
        let (ac, bc) = mock_connectors();

        let exp = Expiries::new_herc20_hbit(Network::Main, start_at, ac.clone(), bc.clone());
        let mut cur = BobState::initial();

        let inc = 1.minutes();

        while cur != BobState::Done {
            inc_connectors(inc, ac.clone(), bc.clone()).await;
            let (want_action, state) = cur.next_herc20_hbit();
            let got_action = exp.next_action_for_bob(cur).await;

            assert_that!(got_action).is_equal_to(want_action);

            cur = state;
        }
    }

    #[tokio::test]
    async fn bob_can_complete_an_hbit_herc20_swap() {
        let start_at = Timestamp::now();
        let (ac, bc) = mock_connectors();

        let exp = Expiries::new_hbit_herc20(Network::Main, start_at, ac.clone(), bc.clone());
        let mut cur = BobState::initial();

        let inc = 1.minutes();

        while cur != BobState::Done {
            inc_connectors(inc, ac.clone(), bc.clone()).await;
            let (want_action, state) = cur.next_hbit_herc20();
            let got_action = exp.next_action_for_bob(cur).await;

            assert_that!(got_action).is_equal_to(want_action);

            cur = state;
        }
    }

    #[tokio::test]
    async fn bob_can_complete_an_herc20_hbit_swap_with_slow_alice_start() {
        let start_at = Timestamp::now();
        let (ac, bc) = mock_connectors();

        let exp = Expiries::new_herc20_hbit(Network::Main, start_at, ac.clone(), bc.clone());
        let inc = 50.minutes(); // Alice takes this long to start.
        inc_connectors(inc, ac.clone(), bc.clone()).await;

        let mut cur = BobState::initial();

        let inc = 1.minutes();

        while cur != BobState::Done {
            inc_connectors(inc, ac.clone(), bc.clone()).await;
            let (want_action, state) = cur.next_herc20_hbit();
            let got_action = exp.next_action_for_bob(cur).await;

            assert_that!(got_action).is_equal_to(want_action);

            cur = state;
        }
    }

    #[tokio::test]
    async fn bob_can_complete_an_hbit_herc20_swap_with_slow_alice_start() {
        let start_at = Timestamp::now();
        let (ac, bc) = mock_connectors();

        let exp = Expiries::new_hbit_herc20(Network::Main, start_at, ac.clone(), bc.clone());
        let inc = 50.minutes(); // Alice takes this long to start.
        inc_connectors(inc, ac.clone(), bc.clone()).await;

        let mut cur = BobState::initial();

        let inc = 1.minutes();

        while cur != BobState::Done {
            inc_connectors(inc, ac.clone(), bc.clone()).await;
            let (want_action, state) = cur.next_hbit_herc20();
            let got_action = exp.next_action_for_bob(cur).await;

            assert_that!(got_action).is_equal_to(want_action);

            cur = state;
        }
    }

    #[tokio::test]
    async fn alice_next_action_wait_to_refund_after_expiry_elapsed() {
        let start_at = Timestamp::now();
        let (ac, bc) = mock_connectors();

        let exp = Expiries::new_herc20_hbit(Network::Main, start_at, ac.clone(), bc.clone());

        let inc = exp.beta_offset.0 + 1.minutes();
        inc_connectors(inc, ac, bc).await;

        let alice_state = AliceState::FundAlphaTransactionBroadcast;

        let want_action = AliceAction::WaitToRefund;
        let got_action = exp.next_action_for_alice(alice_state).await;

        assert_that!(got_action).is_equal_to(want_action);
    }

    #[tokio::test]
    async fn alice_next_action_abort_after_expiry_elapsed() {
        let start_at = Timestamp::now();
        let (ac, bc) = mock_connectors();

        let exp = Expiries::new_herc20_hbit(Network::Main, start_at, ac.clone(), bc.clone());

        let inc = exp.beta_offset.0 + 1.minutes();
        inc_connectors(inc, ac, bc).await;

        let alice_state = AliceState::Started;

        let want_action = AliceAction::Abort;
        let got_action = exp.next_action_for_alice(alice_state).await;

        assert_that!(got_action).is_equal_to(want_action);
    }

    #[tokio::test]
    async fn bob_next_action_refund_after_expiry_elapsed() {
        let start_at = Timestamp::now();
        let (ac, bc) = mock_connectors();

        let exp = Expiries::new_herc20_hbit(Network::Main, start_at, ac.clone(), bc.clone());

        let inc = exp.beta_offset.0 + 1.minutes();
        inc_connectors(inc, ac, bc).await;

        let bob_state = BobState::BetaFunded;

        let want_action = BobAction::Refund;
        let got_action = exp.next_action_for_bob(bob_state).await;

        assert_that!(got_action).is_equal_to(want_action);
    }

    #[tokio::test]
    async fn bob_next_action_abort_after_expiry_elapsed() {
        let start_at = Timestamp::now();
        let (ac, bc) = mock_connectors();

        let exp = Expiries::new_herc20_hbit(Network::Main, start_at, ac.clone(), bc.clone());

        let inc = exp.beta_offset.0 + 1.minutes();
        inc_connectors(inc, ac, bc).await;

        let bob_state = BobState::AlphaFunded;

        let want_action = BobAction::Abort;
        let got_action = exp.next_action_for_bob(bob_state).await;

        assert_that!(got_action).is_equal_to(want_action);
    }

    #[tokio::test]
    async fn bob_next_action_abort_when_alice_takes_too_long_to_start() {
        let start_at = Timestamp::now();
        let (ac, bc) = mock_connectors();

        let exp = Expiries::new_herc20_hbit(Network::Main, start_at, ac.clone(), bc.clone());

        let inc = 2.hours();
        inc_connectors(inc, ac, bc).await;

        let bob_state = BobState::Started;

        let want_action = BobAction::Abort;
        let got_action = exp.next_action_for_bob(bob_state).await;

        assert_that!(got_action).is_equal_to(want_action);
    }

    // Alice tries to attack Bob by waiting until close to beta expiry then
    // broadcasting a transaction with super high fees. Alice hopes that her redeem
    // transaction will go in and that she will be able to refund before Bob has had
    // time to redeem. Bob is still safe for two reasons:
    //
    // 1. He is watching the expiry times and has not yet refunded so we know that
    //    the expiry has only just elapsed.
    // 2. Since the expiry has only just elapsed there is still time enough for
    //    Bob's redeem transaction to reach finality before Alice can refund.
    #[tokio::test]
    async fn bob_next_action_redeem_after_expiry_elapsed_and_secret_broadcast() {
        let start_at = Timestamp::now();
        let (ac, bc) = mock_connectors();

        let exp = Expiries::new_herc20_hbit(Network::Main, start_at, ac.clone(), bc.clone());

        let inc = exp.beta_offset.0 + 1.minutes();
        inc_connectors(inc, ac, bc).await;

        let bob_state = BobState::RedeemBetaTransactionBroadcast;

        let want_action = BobAction::RedeemAlpha;
        let got_action = exp.next_action_for_bob(bob_state).await;

        assert_that!(got_action).is_equal_to(want_action);
    }

    #[tokio::test]
    async fn alice_next_action_after_start_is_deploy_herc20_hbit() {
        let start_at = Timestamp::now();
        let (ac, bc) = mock_connectors();

        let exp = Expiries::new_herc20_hbit(Network::Main, start_at, ac.clone(), bc.clone());

        let alice_state = AliceState::Started;

        let want_action = AliceAction::DeployAlpha;
        let got_action = exp.next_action_for_alice(alice_state).await;

        assert_that!(got_action).is_equal_to(want_action);
    }

    #[tokio::test]
    async fn alice_next_action_after_start_is_fund_hbit_herc20() {
        let start_at = Timestamp::now();
        let (ac, bc) = mock_connectors();

        let exp = Expiries::new_hbit_herc20(Network::Main, start_at, ac.clone(), bc.clone());

        let alice_state = AliceState::Started;

        let want_action = AliceAction::FundAlpha;
        let got_action = exp.next_action_for_alice(alice_state).await;

        assert_that!(got_action).is_equal_to(want_action);
    }

    #[tokio::test]
    async fn bob_next_action_after_alpha_funded_is_deploy_hbit_herc20() {
        let start_at = Timestamp::now();
        let (ac, bc) = mock_connectors();

        let exp = Expiries::new_hbit_herc20(Network::Main, start_at, ac.clone(), bc.clone());

        let bob_state = BobState::AlphaFunded;

        let want_action = BobAction::DeployBeta;
        let got_action = exp.next_action_for_bob(bob_state).await;

        assert_that!(got_action).is_equal_to(want_action);
    }

    #[tokio::test]
    async fn bob_next_action_after_alpha_funded_is_fund_herc20_hbit() {
        let start_at = Timestamp::now();
        let (ac, bc) = mock_connectors();

        let exp = Expiries::new_herc20_hbit(Network::Main, start_at, ac.clone(), bc.clone());

        let bob_state = BobState::AlphaFunded;

        let want_action = BobAction::FundBeta;
        let got_action = exp.next_action_for_bob(bob_state).await;

        assert_that!(got_action).is_equal_to(want_action);
    }
}
