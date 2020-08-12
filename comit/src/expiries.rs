//! Standardized COMIT Expiry Times
//!
//! The COMIT protocols rely on two expiry times in order for the swap to be
//! atomic. One expiry for the alpha ledger and one for the beta ledger. The
//! beta ledger expiry time must elapse before the alpha ledger expiry time.

use crate::timestamp::{self, Timestamp};
use time::{prelude::*, Duration};

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
    /// The alpha ledger relative expiry time.
    alpha_expiry: Duration,
    /// The beta ledger relative expiry time.
    beta_expiry: Duration,
}

impl<A, B> Expiries<A, B>
where
    A: CurrentTime,
    B: CurrentTime,
{
    /// Construct a pair of relative expiries values.
    pub fn new(protocol: Protocol, alpha: A, beta: B) -> Expiries<A, B> {
        let config = protocol.config();

        let alice_needs = happy_path_swap_period_for_alice(&config);
        let bob_needs = happy_path_swap_period_for_bob(&config);

        Expiries {
            config,
            alpha_connector: alpha,
            beta_connector: beta,
            // Bob redeems on alpha ledger so needs time to act before the alpha expiry.
            alpha_expiry: bob_needs,
            // Alice redeems on beta ledger so needs time to act before the beta expiry.
            beta_expiry: alice_needs,
        }
    }

    /// Returns true if both:
    /// 1. Alice has time to transition from initial state to `Done`.
    /// 2. Bob has time to transition from initial state to `Done`.
    pub fn is_useful(&self) -> bool {
        let alice_needs = happy_path_swap_period_for_alice(&self.config);
        let bob_needs = happy_path_swap_period_for_bob(&self.config);

        let alice_has_enough_time = self.beta_expiry >= alice_needs;
        let bob_has_enough_time = self.alpha_expiry >= bob_needs;

        alice_has_enough_time && bob_has_enough_time
    }

    /// Convert relative expiries to absolute expiries.
    pub fn to_absolute(&self) -> (Timestamp, Timestamp) {
        let alpha_now = self.alpha_connector.current_time();
        let alpha_expiry = alpha_now.add_duration(self.alpha_expiry);

        let beta_now = self.beta_connector.current_time();
        let beta_expiry = beta_now.add_duration(self.beta_expiry);

        (alpha_expiry, beta_expiry)
    }

    /// True if Alice has time to complete a swap (i.e. transition to done)
    /// before the expiry time elapses.
    fn alice_can_complete_from(&self, current_state: AliceState, beta_expiry: Timestamp) -> bool {
        let period = period_for_alice_to_complete(&self.config, current_state);
        let now = self.beta_connector.current_time();

        // Alice redeems on beta ledger so is concerned about the beta expiry.
        let end_time = now.add_duration(period);
        end_time < beta_expiry
    }

    /// True if Bob has time to complete a swap (i.e transition to done)
    /// before the expiry time elapses.
    fn bob_can_complete_from(&self, current_state: BobState, alpha_expiry: Timestamp) -> bool {
        let period = period_for_bob_to_complete(&self.config, current_state);
        let now = self.alpha_connector.current_time();

        // Bob redeems on alpha ledger so is concerned about the alpha expiry.
        let end_time = now.add_duration(period);
        end_time < alpha_expiry
    }

    /// If Alice's next action is not taken within X minutes the expiries will
    /// become un-useful. Returns X.
    pub fn alice_should_act_within(
        &self,
        current_state: AliceState,
        beta_expiry: Timestamp,
    ) -> Duration {
        let period = period_for_alice_to_complete(&self.config, current_state);
        let start_time = beta_expiry.sub_duration(period);
        let now = self.beta_connector.current_time();

        timestamp::duration_between(now, start_time)
    }

    /// If Bob's next action is not taken within X minutes the expiries will
    /// become un-useful. Returns X.
    pub fn bob_should_act_within(
        &self,
        current_state: BobState,
        alpha_expiry: Timestamp,
    ) -> Duration {
        let period = period_for_bob_to_complete(&self.config, current_state);
        let start_time = alpha_expiry.sub_duration(period);
        let now = self.alpha_connector.current_time();

        timestamp::duration_between(now, start_time)
    }

    /// Returns the recommended next action that Alice should take.
    pub fn next_action_for_alice(
        &self,
        current_state: AliceState,
        alpha_expiry: Timestamp,
        beta_expiry: Timestamp,
    ) -> AliceAction {
        if current_state == AliceState::Done {
            return AliceAction::NoFurtherAction;
        }

        let funded = current_state.has_sent_fund_transaction();

        if self.alpha_expiry_has_elapsed(alpha_expiry) {
            if funded {
                return AliceAction::Refund;
            }
            return AliceAction::Abort;
        }

        let both_parties_can_complete = self.alice_can_complete_from(current_state, beta_expiry)
            && self.bob_can_complete_from(current_state.into(), alpha_expiry);

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
        alpha_expiry: Timestamp,
        beta_expiry: Timestamp,
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

        if self.beta_expiry_has_elapsed(alpha_expiry) {
            if funded {
                return BobAction::Refund;
            }
            return BobAction::Abort;
        };

        let both_parties_can_complete = self
            .alice_can_complete_from(current_state.into(), beta_expiry)
            && self.bob_can_complete_from(current_state, alpha_expiry);

        if !both_parties_can_complete {
            if funded {
                return BobAction::WaitForRefund;
            }
            return BobAction::Abort;
        }

        let (next_action, _state) = current_state.next();
        next_action
    }

    fn alpha_expiry_has_elapsed(&self, expiry: Timestamp) -> bool {
        let now = self.alpha_connector.current_time();
        now > expiry
    }

    fn beta_expiry_has_elapsed(&self, expiry: Timestamp) -> bool {
        let now = self.beta_connector.current_time();
        now > expiry
    }
}

/// Duration for a complete happy path swap for Alice.
fn happy_path_swap_period_for_alice(config: &Config) -> Duration {
    period_for_alice_to_complete(&config, AliceState::None)
}

/// Duration for a complete happy path swap for Bob.
fn happy_path_swap_period_for_bob(config: &Config) -> Duration {
    period_for_bob_to_complete(&config, BobState::Created)
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

/// Happy path states for Alice.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AliceState {
    /// Initial state, swap not yet created.
    None,
    /// Swap has been created.
    Created,
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
    Create,
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
    /// Gets the next action required to transition to the next state.
    fn next(&self) -> (AliceAction, AliceState) {
        use self::{AliceAction::*, AliceState::*};

        match self {
            None => (Create, Created),
            Created => (FundAlpha, FundAlphaTransactionSent),
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
            Create => {
                // Transition from None to Created
                c.alice_to_act()
            }
            FundAlpha => {
                // Transition from Created to FundAlphaTransactionSent
                c.alice_to_act()
            }
            WaitForFundTransactionFinality => {
                // Transition from FundAlphaTransactionSent to AlphaFunded
                c.mine_alpha_fund_transaction() + c.finality_alpha()
            }
            WaitForBetaFunded => {
                // Transition from AlphaFunded to BetaFunded
                c.bob_to_act() + c.mine_beta_fund_transaction() + c.finality_beta()
            }
            RedeemBeta => {
                // Transition from BetaFunded to RedeemBetaTransactionSent
                c.alice_to_act()
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
            AliceState::None | AliceState::Created => false,
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
    /// Initial state, swap has been created.
    Created,
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
    /// Gets the next action required to transition to the next state.
    fn next(&self) -> (BobAction, BobState) {
        use self::{BobAction::*, BobState::*};

        match self {
            Created => (WaitForAlphaFunded, AlphaFunded),
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
                // Transition from Created to AlphaFunded
                c.alice_to_act() + c.mine_alpha_fund_transaction() + c.finality_alpha()
            }
            FundBeta => {
                // Transition from AlphaFunded to FundBetaTransactionSent
                c.bob_to_act()
            }
            WaitForBetaFundTransactionFinality => {
                // Transition from FundBetaTransactionSent to BetaFunded
                c.mine_beta_fund_transaction() + c.finality_beta()
            }
            WaitForBetaRedeemTransactionToBeSeen => {
                // Transition from BetaFunded to RedeemBetaTransactionSeen
                c.alice_to_act() + c.mine_beta_redeem_transaction()
            }
            RedeemAlpha => {
                // Transition from RedeemBetaTransactionSeen to
                // RedeemAlphaTransactionSent
                c.bob_to_act()
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
            BobState::Created | BobState::AlphaFunded => false,
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
            None => Self::Created,
            Created => Self::Created,
            FundAlphaTransactionSent => Self::Created,
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
            Created => Self::None,
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

// TODO: Add support for lightning.
#[derive(Clone, Copy, Debug)]
pub enum Protocol {
    Herc20Hbit,
    HbitHerc20,
}

impl Protocol {
    fn config(&self) -> Config {
        match self {
            Protocol::Herc20Hbit => Config {
                alpha_required_confirmations: ethereum_required_confirmations(),
                beta_required_confirmations: bitcoin_required_confirmations(),
            },
            Protocol::HbitHerc20 => Config {
                alpha_required_confirmations: bitcoin_required_confirmations(),
                beta_required_confirmations: ethereum_required_confirmations(),
            },
        }
    }
}

/// Configuration values used during transition period calculations.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    alpha_required_confirmations: usize,
    beta_required_confirmations: usize,
}

// TODO: Calculate _real_ values instead of just returning 1 hour.
impl Config {
    /// The duration of time it takes for Alice to act.
    pub fn alice_to_act(&self) -> Duration {
        1.hours()
    }

    /// The duration of time it takes for Bob to act.
    pub fn bob_to_act(&self) -> Duration {
        1.hours()
    }

    /// The duration of time it takes for the alpha fund transaction to be
    /// mined into the blockchain.
    pub fn mine_alpha_fund_transaction(&self) -> Duration {
        1.hours()
    }

    /// The duration of time it takes for the beta fund transaction to be
    /// mined into the blockchain.
    pub fn mine_beta_fund_transaction(&self) -> Duration {
        1.hours()
    }

    /// The duration of time it takes for the alpha redeem transaction to be
    /// mined into the blockchain.
    pub fn mine_alpha_redeem_transaction(&self) -> Duration {
        1.hours()
    }

    /// The duration of time it takes for the beta redeem transaction to be
    /// mined into the blockchain.
    pub fn mine_beta_redeem_transaction(&self) -> Duration {
        1.hours()
    }

    /// The duration of time it takes for a transaction to reach finality on the
    /// alpha ledger.
    pub fn finality_alpha(&self) -> Duration {
        1.hours()
    }

    /// The duration of time it takes for a transaction to reach finality on the
    /// beta ledger.
    pub fn finality_beta(&self) -> Duration {
        1.hours()
    }
}

fn bitcoin_required_confirmations() -> usize {
    // TODO: Add documentation on _why_ we picked this value.
    6
}

fn ethereum_required_confirmations() -> usize {
    // TODO: Add documentation on _why_ we picked this value.
    12
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let (alpha, beta) = mock_connectors();

        let exp = Expiries::new(Protocol::Herc20Hbit, alpha, beta);
        assert!(exp.is_useful());

        let exp = Expiries::new(Protocol::HbitHerc20, alpha, beta);
        assert!(exp.is_useful());
    }
}
