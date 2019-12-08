#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
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

pub mod bitcoin;
pub mod btsieve;
pub mod comit_api;
pub mod config;
pub mod ethereum;
pub mod first_or_else;
pub mod http_api;
pub mod load_swaps;
pub mod logging;
pub mod network;
#[cfg(test)]
pub mod quickcheck;
pub mod seed;
#[cfg(test)]
pub mod spectral_ext;
pub mod swap_protocols;
pub mod timestamp;

use crate::swap_protocols::{
    asset::Asset,
    rfc003::{events::HtlcEvents, Ledger},
};
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

pub trait CreateLedgerEvents<L: Ledger, A: Asset> {
    fn create_ledger_events(&self) -> Box<dyn HtlcEvents<L, A>>;
}
