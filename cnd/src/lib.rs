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
pub mod load_swaps;
#[macro_use]
pub mod network;
#[cfg(test)]
pub mod quickcheck;
#[macro_use]
pub mod seed;
#[cfg(test)]
pub mod spectral_ext;
pub mod swap_protocols;
pub mod timestamp;

use anyhow::Context;
use directories::ProjectDirs;
use std::path::{Path, PathBuf};

lazy_static::lazy_static! {
    pub static ref SECP: ::bitcoin::secp256k1::Secp256k1<::bitcoin::secp256k1::All> =
        ::bitcoin::secp256k1::Secp256k1::new();
}

// Linux: /home/<user>/.config/comit/
// Windows: C:\Users\<user>\AppData\Roaming\comit\config\
// OSX: /Users/<user>/Library/Preferences/comit/
fn config_dir() -> Option<PathBuf> {
    ProjectDirs::from("", "", "comit").map(|proj_dirs| proj_dirs.config_dir().to_path_buf())
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
    ProjectDirs::from("", "", "comit").map(|proj_dirs| proj_dirs.data_dir().to_path_buf())
}

pub type Never = std::convert::Infallible;
