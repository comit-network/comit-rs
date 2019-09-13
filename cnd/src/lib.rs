#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![forbid(unsafe_code)]

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

#[macro_use]
pub mod libp2p_comit_ext;

pub mod btsieve;
pub mod comit_api;
pub mod comit_i_routes;
pub mod config;
pub mod db;
pub mod http_api;
pub mod logging;
pub mod metadata_store;
pub mod network;
pub mod seed;
pub mod state_store;
pub mod std_ext;
pub mod swap_protocols;

use directories::ProjectDirs;
use std::path::PathBuf;

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
