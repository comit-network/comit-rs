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

pub mod bitcoin;
pub mod comit_api;
pub mod comit_client;
pub mod comit_i_routes;
pub mod config;
pub mod db;
pub mod http_api;
pub mod logging;
pub mod network;
pub mod seed;
pub mod std_ext;
pub mod stream_ext;
pub mod swap_protocols;

use directories::ProjectDirs;
use std::path::PathBuf;

use bitcoin_support::bitcoin::secp256k1;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref SECP: secp256k1::Secp256k1<secp256k1::All> = secp256k1::Secp256k1::new();
}

// Linux: /home/<user>/.config/comit/
// Windows: C:\Users\<user>\AppData\Roaming\comit\config\
// OSX: /Users/<user>/Library/Preferences/comit/
fn config_dir() -> Option<PathBuf> {
    ProjectDirs::from("", "", "comit").map(|proj_dirs| proj_dirs.config_dir().to_path_buf())
}

// Linux: /home/<user>/.local/share/comit/
// Windows: C:\Users\<user>\AppData\Roaming\comit\
// OSX: /Users/<user>/Library/Application Support/comit/
fn data_dir() -> Option<PathBuf> {
    ProjectDirs::from("", "", "comit").map(|proj_dirs| proj_dirs.data_dir().to_path_buf())
}
