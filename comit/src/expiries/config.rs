//! This module provides functionality for calculating the duration of actions
//! required to determine the transition period from one swap state to the next.

use std::fmt;
use time::{prelude::*, Duration};

// TODO: From somewhere within the system we need to return to the
// user a transaction fee to use for each of the transactions (deploy,
// fund, refund, redeem). In this module we assume the fee is set to
// the suggested amount. We rely on this assumption for the
// confirmation time calculations in this module to be correct.
//
// For Ethereum:
//
// gas limit: blockchain-contracts already implements gas limit functionality
//            for each Ethereum transaction type.
// gas price: current gas price can be learned from the Ethereum blockchain,
//            for example from geth or infura.
// modifier:  We could use a pre-configured modifier, as is widely done in the
//            Ethereum ecosystem for slow, medium, fast.
//
// For Bitcoin:
//
// With a configured 'include within N blocks' we can use bitcoind via
// bitcoin-cli, by calling:
//
//  `bitcoin-cli estimatesmartfee N_BLOCKS`
//
// CAVEAT: The fee estimator relies on an active mempool with good uptime to
// watch fee activity on the network, that in turn implies that our calculations
// herein are only as good as the fee estimator of the Bitcoin connector.

// We use specific integer types to limit the upper bound, this reduces the need
// to turn off lints.

const BITCOIN_BLOCK_TIME_SECS: u16 = 600; // 10 minutes, average Bitcoin block time.
const ETHEREUM_BLOCK_TIME_SECS: u16 = 20; // Conservative Ethereum block time.

const BITCOIN_MINE_WITHIN_N_BLOCKS: u8 = 3; // Value arbitrarily chosen.
const ETHEREUM_MINE_WITHIN_N_BLOCKS: u8 = 3; // Value arbitrarily chosen.

const BITCOIN_CONFIRMATIONS: u8 = 6; // Standard in the Bitcoin ecosystem.
const ETHEREUM_CONFIRMATIONS: u8 = 30; // Value used by Kraken.

const ACT_IN_SOFTWARE_MINS: u32 = 15;
const ACT_WITH_USER_INTERVENTION_MINS: u32 = 60;

// TODO: Add support for lightning.
#[derive(Clone, Copy, Debug)]
pub enum Protocol {
    Herc20Hbit,
    HbitHerc20,
}

impl Protocol {
    pub fn config(&self) -> Config {
        match self {
            Protocol::Herc20Hbit => Config {
                alpha_required_confirmations: ethereum_required_confirmations(),
                beta_required_confirmations: bitcoin_required_confirmations(),
                alpha_block_time: ETHEREUM_BLOCK_TIME_SECS,
                beta_block_time: BITCOIN_BLOCK_TIME_SECS,
                alpha_mine_fund_within_n_blocks: ETHEREUM_MINE_WITHIN_N_BLOCKS,
                beta_mine_fund_within_n_blocks: BITCOIN_MINE_WITHIN_N_BLOCKS,
                alpha_mine_redeem_within_n_blocks: ETHEREUM_MINE_WITHIN_N_BLOCKS,
                beta_mine_redeem_within_n_blocks: BITCOIN_MINE_WITHIN_N_BLOCKS,
            },
            Protocol::HbitHerc20 => Config {
                alpha_required_confirmations: bitcoin_required_confirmations(),
                beta_required_confirmations: ethereum_required_confirmations(),
                alpha_block_time: BITCOIN_BLOCK_TIME_SECS,
                beta_block_time: ETHEREUM_BLOCK_TIME_SECS,
                alpha_mine_fund_within_n_blocks: BITCOIN_MINE_WITHIN_N_BLOCKS,
                beta_mine_fund_within_n_blocks: ETHEREUM_MINE_WITHIN_N_BLOCKS,
                alpha_mine_redeem_within_n_blocks: BITCOIN_MINE_WITHIN_N_BLOCKS,
                beta_mine_redeem_within_n_blocks: ETHEREUM_MINE_WITHIN_N_BLOCKS,
            },
        }
    }
}

/// Configuration values used during transition period calculations.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    alpha_required_confirmations: u8,
    beta_required_confirmations: u8,
    alpha_block_time: u16,
    beta_block_time: u16,
    alpha_mine_fund_within_n_blocks: u8,
    beta_mine_fund_within_n_blocks: u8,
    alpha_mine_redeem_within_n_blocks: u8,
    beta_mine_redeem_within_n_blocks: u8,
}

impl Config {
    /// The duration of time it takes for Alice to start.
    pub fn start(&self) -> Duration {
        period_to_act_with_user_interaction()
    }

    /// The duration of time it takes to create the alpha fund transaction.
    pub fn create_alpha_fund_transaction(&self) -> Duration {
        period_to_act_with_user_interaction()
    }

    /// The duration of time it takes to create the beta fund transaction.
    pub fn create_beta_fund_transaction(&self) -> Duration {
        period_to_act_in_software()
    }

    /// The duration of time it takes to create the alpha redeem transaction.
    pub fn create_alpha_redeem_transaction(&self) -> Duration {
        period_to_act_in_software()
    }

    /// The duration of time it takes to create the beta redeem transaction.
    pub fn create_beta_redeem_transaction(&self) -> Duration {
        period_to_act_with_user_interaction()
    }

    /// The duration of time it takes for the alpha fund transaction to be
    /// mined into the blockchain.
    pub fn mine_alpha_fund_transaction(&self) -> Duration {
        let n = self.alpha_mine_fund_within_n_blocks;
        let block_time = self.alpha_block_time;

        time_to_mine_n_blocks(n, block_time)
    }

    /// The duration of time it takes for the beta fund transaction to be
    /// mined into the blockchain.
    pub fn mine_beta_fund_transaction(&self) -> Duration {
        let n = self.beta_mine_fund_within_n_blocks;
        let block_time = self.beta_block_time;

        time_to_mine_n_blocks(n, block_time)
    }

    /// The duration of time it takes for the alpha redeem transaction to be
    /// mined into the blockchain.
    pub fn mine_alpha_redeem_transaction(&self) -> Duration {
        let n = self.alpha_mine_redeem_within_n_blocks;
        let block_time = self.alpha_block_time;

        time_to_mine_n_blocks(n, block_time)
    }

    /// The duration of time it takes for the beta redeem transaction to be
    /// mined into the blockchain.
    pub fn mine_beta_redeem_transaction(&self) -> Duration {
        let n = self.beta_mine_redeem_within_n_blocks;
        let block_time = self.beta_block_time;

        time_to_mine_n_blocks(n, block_time)
    }

    /// The duration of time it takes for a transaction to reach finality on the
    /// alpha ledger.
    pub fn finality_alpha(&self) -> Duration {
        let n = self.alpha_required_confirmations;
        let block_time = self.alpha_block_time;

        time_to_mine_n_blocks(n, block_time)
    }

    /// The duration of time it takes for a transaction to reach finality on the
    /// beta ledger.
    pub fn finality_beta(&self) -> Duration {
        let n = self.beta_required_confirmations;
        let block_time = self.beta_block_time;

        time_to_mine_n_blocks(n, block_time)
    }
}

/// If some action requires only software give the counterparty this long to
/// act. 15 minutes to allow for network congestion etc.
pub fn period_to_act_in_software() -> Duration {
    ACT_IN_SOFTWARE_MINS.minutes()
}

/// If some action requires user input give the counterparty this long to
/// act.
pub fn period_to_act_with_user_interaction() -> Duration {
    ACT_WITH_USER_INTERVENTION_MINS.minutes()
}

fn bitcoin_required_confirmations() -> u8 {
    BITCOIN_CONFIRMATIONS
}

fn ethereum_required_confirmations() -> u8 {
    ETHEREUM_CONFIRMATIONS
}

// Time to mine n blocks is governed by a Poisson distribution. As an
// improvement we could calculate that instead of using this naive
// implementation. For more details see:
// - https://en.wikipedia.org/wiki/Poisson_distribution
// - https://www.reddit.com/r/btc/comments/6v5ee7/block_times_and_probabilities/
fn time_to_mine_n_blocks(n: u8, average_block_time_secs: u16) -> Duration {
    let t = n as u16 * average_block_time_secs;
    Duration::seconds(t as i64)
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pretty = format!(
            r#"
    alpha_required_confirmations: {}
    beta_required_confirmations: {}
    alpha_block_time: {}
    beta_block_time: {}
    alpha_mine_fund_within_n_blocks {}
    beta_mine_fund_within_n_blocks {}
    alpha_mine_redeem_within_n_blocks {}
    beta_mine_redeem_within_n_blocks {}
"#,
            self.alpha_required_confirmations,
            self.beta_required_confirmations,
            self.alpha_block_time,
            self.beta_block_time,
            self.alpha_mine_fund_within_n_blocks,
            self.beta_mine_fund_within_n_blocks,
            self.alpha_mine_redeem_within_n_blocks,
            self.beta_mine_redeem_within_n_blocks,
        );

        write!(f, "{}", pretty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectral::prelude::*;

    #[test]
    fn time_to_mine_n_blocks_bitcoin() {
        let n = bitcoin_required_confirmations();
        let block_time = BITCOIN_BLOCK_TIME_SECS;

        let max = 5.hours(); // Arbitrarily chosen ceiling.
        let time = time_to_mine_n_blocks(n, block_time);

        assert_that!(time).is_less_than(max)
    }
}
