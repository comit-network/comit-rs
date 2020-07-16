#![warn(
    unused_extern_crates,
    missing_debug_implementations,
    missing_copy_implementations,
    rust_2018_idioms,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::fallible_impl_from,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::dbg_macro
)]
#![allow(dead_code)] // To be removed further down the line
#![forbid(unsafe_code)]
// TODO: Add no unwrap policy

pub mod bitcoin;
pub mod config;
pub mod ethereum;
pub mod float_maths;
pub mod history;
pub mod jsonrpc;
pub mod maker;
pub mod mid_market_rate;
pub mod network;
pub mod ongoing_takers;
pub mod options;
pub mod order;
pub mod rate;
pub mod seed;
pub mod swap;
pub mod swap_id;

use anyhow::Context;
use conquer_once::Lazy;
pub use maker::Maker;
pub use mid_market_rate::MidMarketRate;
pub use ongoing_takers::PeersWithOngoingTrades;
pub use rate::{Rate, Spread};
pub use seed::Seed;
use std::path::{Path, PathBuf};
pub use swap_id::SwapId;

pub static SECP: Lazy<::bitcoin::secp256k1::Secp256k1<::bitcoin::secp256k1::All>> =
    Lazy::new(::bitcoin::secp256k1::Secp256k1::new);

/// This is to store the configuration and seed files
// Linux: /home/<user>/.config/nectar/
// OSX: /Users/<user>/Library/Preferences/nectar/
fn config_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "nectar")
        .map(|proj_dirs| proj_dirs.config_dir().to_path_buf())
}

pub fn default_config_path() -> anyhow::Result<PathBuf> {
    crate::config_dir()
        .map(|dir| Path::join(&dir, "config.toml"))
        .context("Could not generate default configuration path")
}

/// This is to store the DB
// Linux: /home/<user>/.local/share/nectar/
// OSX: /Users/<user>/Library/Application Support/nectar/
pub fn data_dir() -> Option<std::path::PathBuf> {
    directories::ProjectDirs::from("", "", "nectar")
        .map(|proj_dirs| proj_dirs.data_dir().to_path_buf())
}

pub fn ensure_directory_exists(file: &Path) -> Result<(), std::io::Error> {
    if let Some(path) = file.parent() {
        if !path.exists() {
            tracing::info!(
                "Parent directory does not exist, creating recursively: {}",
                file.display()
            );
            return std::fs::create_dir_all(path);
        }
    }
    Ok(())
}

#[derive(Debug, Copy, Clone, strum_macros::Display)]
#[strum(serialize_all = "UPPERCASE")]
pub enum Symbol {
    Btc,
    Dai,
}

#[cfg(all(test, feature = "test-docker"))]
pub mod test_harness;

#[cfg(test)]
mod tests {
    use crate::Symbol;

    #[test]
    fn symbol_serializes_correctly() {
        let btc = Symbol::Btc;
        let dai = Symbol::Dai;

        assert_eq!(String::from("BTC"), btc.to_string());
        assert_eq!(String::from("DAI"), dai.to_string());
    }
}
