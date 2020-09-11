//! This module provides functionality for calculating the duration of actions
//! required to determine the transition period from one swap state to the next.

use crate::Network;
use std::fmt;
use time::Duration;

// TODO: From somewhere within the system we need to return to the user a
// transaction fee to use for each of the transactions (deploy, fund, refund,
// redeem). In this module we assume the fee is set to the suggested amount. We
// rely on this assumption for the confirmation time calculations to be correct.
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

mod main {
    pub const BITCOIN_BLOCK_TIME_SECS: u16 = 600; // 10 minutes, average Bitcoin block time.
    pub const ETHEREUM_BLOCK_TIME_SECS: u16 = 20; // Conservative Ethereum block
                                                  // time.

    pub const BITCOIN_CONFIRMATIONS: u8 = 6; // Standard in the Bitcoin ecosystem.
    pub const ETHEREUM_CONFIRMATIONS: u8 = 30; // Value used by Kraken.

    pub const ACT_IN_SOFTWARE_SECS: u32 = 15 * 60; // Value arbitrarily chosen.
    pub const ACT_WITH_USER_INTERACTION_SECS: u32 = 60 * 60; // Value arbitrarily
                                                             // chosen.
}

mod dev {
    // The local dev nets in the e2e tests have a block time of 1 second.
    pub const BITCOIN_BLOCK_TIME_SECS: u16 = 1;
    pub const ETHEREUM_BLOCK_TIME_SECS: u16 = 1;

    // We don't need many confirmations during testing ...
    pub const BITCOIN_CONFIRMATIONS: u8 = 1;
    pub const ETHEREUM_CONFIRMATIONS: u8 = 1;

    // The e2e tests act very fast :)
    pub const ACT_IN_SOFTWARE_SECS: u32 = 1;
    pub const ACT_WITH_USER_INTERACTION_SECS: u32 = 1;
}

// TODO: Fee recommendation must use this value of N.
const BITCOIN_MINE_WITHIN_N_BLOCKS: u8 = 3; // Value arbitrarily chosen.
const ETHEREUM_MINE_WITHIN_N_BLOCKS: u8 = 3; // Value arbitrarily chosen.

/// Configuration values used during transition period calculations.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    protocol: Protocol,
    alpha_required_confirmations: u8,
    beta_required_confirmations: u8,
    alpha_average_block_time: u16,
    beta_average_block_time: u16,
    alpha_mine_deploy_within_n_blocks: u8,
    beta_mine_deploy_within_n_blocks: u8,
    alpha_mine_fund_within_n_blocks: u8,
    beta_mine_fund_within_n_blocks: u8,
    alpha_mine_redeem_within_n_blocks: u8,
    beta_mine_redeem_within_n_blocks: u8,
    act_in_software: u32,
    act_with_user_interaction: u32,
}

impl Config {
    /// Construct a config object suitable for a herc20-hbit swap.
    pub const fn herc20_hbit(network: Network) -> Self {
        Config {
            protocol: Protocol::Herc20Hbit,
            alpha_required_confirmations: ethereum_confirmations(network),
            beta_required_confirmations: bitcoin_confirmations(network),
            alpha_average_block_time: ethereum_blocktime(network),
            beta_average_block_time: bitcoin_blocktime(network),
            alpha_mine_deploy_within_n_blocks: ETHEREUM_MINE_WITHIN_N_BLOCKS,
            beta_mine_deploy_within_n_blocks: BITCOIN_MINE_WITHIN_N_BLOCKS,
            alpha_mine_fund_within_n_blocks: ETHEREUM_MINE_WITHIN_N_BLOCKS,
            beta_mine_fund_within_n_blocks: BITCOIN_MINE_WITHIN_N_BLOCKS,
            alpha_mine_redeem_within_n_blocks: ETHEREUM_MINE_WITHIN_N_BLOCKS,
            beta_mine_redeem_within_n_blocks: BITCOIN_MINE_WITHIN_N_BLOCKS,
            act_in_software: act_in_software(network),
            act_with_user_interaction: act_with_user_interaction(network),
        }
    }

    /// Construct a config object suitable for a hbit-herc20 swap.
    pub const fn hbit_herc20(network: Network) -> Self {
        Config {
            protocol: Protocol::HbitHerc20,
            alpha_required_confirmations: bitcoin_confirmations(network),
            beta_required_confirmations: ethereum_confirmations(network),
            alpha_average_block_time: bitcoin_blocktime(network),
            beta_average_block_time: ethereum_blocktime(network),
            alpha_mine_deploy_within_n_blocks: BITCOIN_MINE_WITHIN_N_BLOCKS,
            beta_mine_deploy_within_n_blocks: ETHEREUM_MINE_WITHIN_N_BLOCKS,
            alpha_mine_fund_within_n_blocks: BITCOIN_MINE_WITHIN_N_BLOCKS,
            beta_mine_fund_within_n_blocks: ETHEREUM_MINE_WITHIN_N_BLOCKS,
            alpha_mine_redeem_within_n_blocks: BITCOIN_MINE_WITHIN_N_BLOCKS,
            beta_mine_redeem_within_n_blocks: ETHEREUM_MINE_WITHIN_N_BLOCKS,
            act_in_software: act_in_software(network),
            act_with_user_interaction: act_with_user_interaction(network),
        }
    }

    /// Alpha/beta expiries are required to be separated by at least this window
    /// in order for Bobs redeem transaction to be 'safe' i.e., for Bob to be
    /// sure Alice can not redeem and refund at the same time (e.g. by trying to
    /// front run his redeem transaction).
    pub fn bobs_safety_window(&self) -> Duration {
        self.broadcast_alpha_redeem_transaction()
            + self.mine_alpha_redeem_transaction()
            + self.finality_alpha()
    }

    /// The duration of time it takes for Alice to start.
    pub const fn start(&self) -> Duration {
        self.period_to_act_with_user_interaction()
    }

    /// The duration of time it takes to broadcast the alpha deploy transaction.
    pub const fn broadcast_alpha_deploy_transaction(&self) -> Duration {
        self.period_to_act_with_user_interaction()
    }

    /// The duration of time it takes to broadcast the beta deploy transaction.
    pub const fn broadcast_beta_deploy_transaction(&self) -> Duration {
        self.period_to_act_in_software()
    }

    /// The duration of time it takes to broadcast the alpha fund transaction.
    pub const fn broadcast_alpha_fund_transaction(&self) -> Duration {
        self.period_to_act_with_user_interaction()
    }

    /// The duration of time it takes to broadcast the beta fund transaction.
    pub const fn broadcast_beta_fund_transaction(&self) -> Duration {
        self.period_to_act_in_software()
    }

    /// The duration of time it takes to broadcast the alpha redeem transaction.
    pub const fn broadcast_alpha_redeem_transaction(&self) -> Duration {
        self.period_to_act_in_software()
    }

    /// The duration of time it takes to broadcast the beta redeem transaction.
    pub const fn broadcast_beta_redeem_transaction(&self) -> Duration {
        self.period_to_act_with_user_interaction()
    }

    /// The duration of time we should wait to ensure that the alpha deploy
    /// transaction has been mined into the blockchain.
    pub const fn mine_alpha_deploy_transaction(&self) -> Duration {
        let n = self.alpha_mine_deploy_within_n_blocks;
        let block_time = self.alpha_average_block_time;

        time_to_mine_n_blocks(n, block_time)
    }

    /// The duration of time we should wait to ensure that the beta deploy
    /// transaction has been mined into the blockchain.
    pub const fn mine_beta_deploy_transaction(&self) -> Duration {
        let n = self.beta_mine_deploy_within_n_blocks;
        let block_time = self.beta_average_block_time;

        time_to_mine_n_blocks(n, block_time)
    }

    /// The duration of time we should wait to ensure that the alpha fund
    /// transaction has been mined into the blockchain.
    pub const fn mine_alpha_fund_transaction(&self) -> Duration {
        let n = self.alpha_mine_fund_within_n_blocks;
        let block_time = self.alpha_average_block_time;

        time_to_mine_n_blocks(n, block_time)
    }

    /// The duration of time we should wait to ensure that the beta fund
    /// transaction has been mined into the blockchain.
    pub const fn mine_beta_fund_transaction(&self) -> Duration {
        let n = self.beta_mine_fund_within_n_blocks;
        let block_time = self.beta_average_block_time;

        time_to_mine_n_blocks(n, block_time)
    }

    /// The duration of time we should wait to ensure that the alpha redeem
    /// transaction has been mined into the blockchain.
    pub const fn mine_alpha_redeem_transaction(&self) -> Duration {
        let n = self.alpha_mine_redeem_within_n_blocks;
        let block_time = self.alpha_average_block_time;

        time_to_mine_n_blocks(n, block_time)
    }

    /// The duration of time we should wait to ensure that the beta redeem
    /// transaction has been mined into the blockchain.
    pub const fn mine_beta_redeem_transaction(&self) -> Duration {
        let n = self.beta_mine_redeem_within_n_blocks;
        let block_time = self.beta_average_block_time;

        time_to_mine_n_blocks(n, block_time)
    }

    /// The duration of time we should wait to ensure a transaction has reached
    /// finality on the alpha ledger.
    pub const fn finality_alpha(&self) -> Duration {
        let n = self.alpha_required_confirmations;
        let block_time = self.alpha_average_block_time;

        time_to_mine_n_blocks(n, block_time)
    }

    /// The duration of time we should wait to ensure a transaction has reached
    /// finality on the beta ledger.
    pub const fn finality_beta(&self) -> Duration {
        let n = self.beta_required_confirmations;
        let block_time = self.beta_average_block_time;

        time_to_mine_n_blocks(n, block_time)
    }

    /// If some action requires only software give the actor this long to
    /// act.
    pub const fn period_to_act_in_software(&self) -> Duration {
        Duration::seconds(self.act_in_software as i64)
    }

    /// If some action requires user interaction give the actor this long
    /// to act.
    pub const fn period_to_act_with_user_interaction(&self) -> Duration {
        Duration::seconds(self.act_with_user_interaction as i64)
    }

    /// Gets the protocol for this config object.
    pub const fn protocol(&self) -> Protocol {
        self.protocol
    }
}

// Time to mine N blocks is governed by a Poisson distribution. We do not,
// however, calculate the Poisson distribution. Instead we use the naive method
// of multiplying average block time by the number of required confirmations.
// This is satisfactory because both the naive method of calculation and the
// Poisson distribution calculation rely on the average block time (i.e. average
// number of events with the time period). Since this value is the major source
// of error, and both calculation methods contain this error, which calculation
// method we use is not important.
//
// For more details on Poisson distribution and how this relates to time to mine
// N blocks please see:
// - https://en.wikipedia.org/wiki/Poisson_distribution
// - https://www.reddit.com/r/btc/comments/6v5ee7/block_times_and_probabilities/
const fn time_to_mine_n_blocks(n: u8, average_block_time_secs: u16) -> Duration {
    let t = n as u16 * average_block_time_secs;

    // Because of the nature of the Poisson distribution of events the probability
    // of at least N events within time T is not high enough for our purposes. This
    // time window is the primary source of safety for the COMIT protocol. Since N
    // blocks is a hard limit users will most certainly want to wait for the N
    // blocks to be mined. Therefore it is likely that when block time exceeds
    // average block time an actor waits longer than time T, this will cause the
    // swap to abort. Therefore we double the time T.
    //
    // In the future we could define an acceptable probability threshold and
    // actually do the math to calculate this time window. This adds however a lot
    // of complexity for minimal benefit.

    let acceptable = t as i64 * 2;

    Duration::seconds(acceptable)
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pretty = format!(
            r#"
    alpha_required_confirmations: {}
    beta_required_confirmations: {}
    alpha_average_block_time: {}
    beta_average_block_time: {}
    alpha_mine_fund_within_n_blocks {}
    beta_mine_fund_within_n_blocks {}
    alpha_mine_redeem_within_n_blocks {}
    beta_mine_redeem_within_n_blocks {}
    act_in_software {}
    act_with_user_interaction {}
"#,
            self.alpha_required_confirmations,
            self.beta_required_confirmations,
            self.alpha_average_block_time,
            self.beta_average_block_time,
            self.alpha_mine_fund_within_n_blocks,
            self.beta_mine_fund_within_n_blocks,
            self.alpha_mine_redeem_within_n_blocks,
            self.beta_mine_redeem_within_n_blocks,
            self.act_in_software,
            self.act_with_user_interaction,
        );

        write!(f, "{}", pretty)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Protocol {
    Herc20Hbit,
    HbitHerc20,
}

const fn bitcoin_blocktime(network: Network) -> u16 {
    match network {
        Network::Main | Network::Test => main::BITCOIN_BLOCK_TIME_SECS,
        Network::Dev => dev::BITCOIN_BLOCK_TIME_SECS,
    }
}

const fn bitcoin_confirmations(network: Network) -> u8 {
    match network {
        Network::Main | Network::Test => main::BITCOIN_CONFIRMATIONS,
        Network::Dev => dev::BITCOIN_CONFIRMATIONS,
    }
}

const fn ethereum_blocktime(network: Network) -> u16 {
    match network {
        Network::Main | Network::Test => main::ETHEREUM_BLOCK_TIME_SECS,
        Network::Dev => dev::ETHEREUM_BLOCK_TIME_SECS,
    }
}

const fn ethereum_confirmations(network: Network) -> u8 {
    match network {
        Network::Main | Network::Test => main::ETHEREUM_CONFIRMATIONS,
        Network::Dev => dev::ETHEREUM_CONFIRMATIONS,
    }
}

const fn act_in_software(network: Network) -> u32 {
    match network {
        Network::Main | Network::Test => main::ACT_IN_SOFTWARE_SECS,
        Network::Dev => dev::ACT_IN_SOFTWARE_SECS,
    }
}

const fn act_with_user_interaction(network: Network) -> u32 {
    match network {
        Network::Main | Network::Test => main::ACT_WITH_USER_INTERACTION_SECS,
        Network::Dev => dev::ACT_WITH_USER_INTERACTION_SECS,
    }
}
