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
#[macro_use]
pub mod network;
#[cfg(test)]
pub mod proptest;
#[cfg(test)]
pub mod quickcheck;
#[macro_use]
mod seed;
#[cfg(test)]
pub mod spectral_ext;

pub mod comit_api;
pub mod config;
pub mod connectors;
pub mod file_lock;
pub mod http_api;
pub mod init_swap;
pub mod load_swaps;
pub mod protocol_spawner;
pub mod respawn;
pub mod storage;
mod swap_id;
pub mod swap_protocols;
mod tracing_ext;

use anyhow::Context;
use std::{
    env,
    path::{Path, PathBuf},
};

pub use self::{seed::*, swap_id::*};
// Export comit types so we do not need to worry about where they come from.
pub use comit::{RelativeTime, Role, Secret, SecretHash, Side, Timestamp};

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

/// Returns `/Users/[username]/Library/Application Support/Lnd/` for macos.
/// Returns `%LOCALAPPDATA%/Lnd for windows.
/// Returns `~/.lnd` if $HOME exists for linux.
pub fn lnd_default_dir() -> Option<PathBuf> {
    if cfg!(target_os = "macos") || cfg!(target_os = "windows") {
        directories::ProjectDirs::from("", "", "Lnd")
            .map(|proj_dirs| proj_dirs.data_dir().to_path_buf())
    } else if cfg!(target_os = "linux") {
        directories::UserDirs::new().map(|d| d.home_dir().to_path_buf().join(".lnd"))
    } else {
        None
    }
}

/// Returns the directory used by lnd.
pub fn lnd_dir() -> Option<PathBuf> {
    if let Ok(dir) = env::var("LND_DIR") {
        return Some(PathBuf::from(dir));
    }
    lnd_default_dir()
}

pub mod htlc_location {
    pub use comit::htlc_location::*;
}

pub mod identity {
    pub use comit::identity::*;
}

pub mod transaction {
    pub use comit::transaction::*;
}

pub mod asset {
    pub use comit::asset::*;
    use derivative::Derivative;

    #[derive(Clone, Derivative, PartialEq)]
    #[derivative(Debug = "transparent")]
    pub enum AssetKind {
        Bitcoin(Bitcoin),
        Ether(Ether),
        Erc20(Erc20),
    }

    impl From<Bitcoin> for AssetKind {
        fn from(amount: Bitcoin) -> Self {
            AssetKind::Bitcoin(amount)
        }
    }

    impl From<Ether> for AssetKind {
        fn from(quantity: Ether) -> Self {
            AssetKind::Ether(quantity)
        }
    }

    impl From<Erc20> for AssetKind {
        fn from(quantity: Erc20) -> Self {
            AssetKind::Erc20(quantity)
        }
    }
}

pub mod ethereum {
    pub use comit::ethereum::*;
}

pub mod bitcoin {
    pub use comit::bitcoin::PublicKey;
}

pub mod lightning {
    pub use comit::lightning::PublicKey;
}

pub mod btsieve {
    pub use comit::btsieve::*;
}
