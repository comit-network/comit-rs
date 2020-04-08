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
    clippy::print_stdout,
    clippy::dbg_macro
)]
#![forbid(unsafe_code)]

// Cannot do `#[strum_discriminants(derive(strum_macros::EnumString))]` at the
// moment. Hence we need to `#[macro_use]` in order to derive strum macros on
// an enum created by `strum_discriminants`.
#[macro_use]
extern crate strum_macros;

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

#[macro_use]
pub mod libp2p_comit_ext;
#[macro_use]
pub mod db;

pub mod asset;
pub mod bitcoin;
pub mod btsieve;
pub mod comit_api;
pub mod config;
pub mod ethereum;
pub mod http_api;
pub mod init_swap;
pub mod lightning;
pub mod lnd;
pub mod load_swaps;
#[macro_use]
pub mod network;
#[cfg(test)]
pub mod quickcheck;
#[macro_use]
pub mod seed;
pub mod file_lock;
pub mod jsonrpc;
#[cfg(test)]
pub mod spectral_ext;
pub mod swap_protocols;
pub mod timestamp;

use anyhow::Context;
use std::{
    env,
    path::{Path, PathBuf},
};

/// Define domain specific terms using identity module so that we can refer to
/// things in an ergonomic fashion e.g., `identity::Bitcoin`.
pub mod identity {
    pub use crate::{
        bitcoin::PublicKey as Bitcoin, ethereum::Address as Ethereum,
        lightning::PublicKey as Lightning,
    };
}

/// Define domain specific terms using transaction module so that we can refer
/// to things in an ergonomic fashion e.g., `transaction::Ethereum`.
pub mod transaction {
    pub use crate::ethereum::Transaction as Ethereum;
    pub use bitcoin::Transaction as Bitcoin;
}

/// Define domain specific terms using htlc_location module so that we can refer
/// to things in an ergonomic fashion e.g., `htlc_location::Bitcoin`.
pub mod htlc_location {
    pub use crate::ethereum::Address as Ethereum;
    pub use bitcoin::OutPoint as Bitcoin;
}

lazy_static::lazy_static! {
    pub static ref SECP: ::bitcoin::secp256k1::Secp256k1<::bitcoin::secp256k1::All> =
        ::bitcoin::secp256k1::Secp256k1::new();
}

// Linux: /home/<user>/.config/comit/
// Windows: C:\Users\<user>\AppData\Roaming\comit\config\
// OSX: /Users/<user>/Library/Preferences/comit/
fn config_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "comit")
        .map(|proj_dirs| proj_dirs.config_dir().to_path_buf())
}

pub fn default_config_path() -> anyhow::Result<PathBuf> {
    crate::config_dir()
        .map(|dir| Path::join(&dir, "cnd.toml"))
        .context("Could not generate default configuration path")
}

// Linux: /home/<user>/.local/share/comit/
// Windows: C:\Users\<user>\AppData\Roaming\comit\
// OSX: /Users/<user>/Library/Application Support/comit/
pub fn data_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "comit")
        .map(|proj_dirs| proj_dirs.data_dir().to_path_buf())
}

/// Returns `/Users/[username]/Library/Application Support/Lnd/`.
/// exists.
#[cfg(target_os = "macos")]
pub fn lnd_default_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "Lnd")
        .map(|proj_dirs| proj_dirs.data_dir().to_path_buf())
}

/// Returns `~/.lnd` if $HOME exists.
#[cfg(target_os = "linux")]
pub fn lnd_default_dir() -> Option<PathBuf> {
    directories::UserDirs::new().map(|d| d.home_dir().to_path_buf().join(".lnd"))
}

/// Returns the directory used by lnd.
pub fn lnd_dir() -> Option<PathBuf> {
    if let Ok(dir) = env::var("LND_DIR") {
        return Some(PathBuf::from(dir));
    }
    lnd_default_dir()
}

pub type Never = std::convert::Infallible;
