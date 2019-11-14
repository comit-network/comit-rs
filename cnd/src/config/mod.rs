#![allow(clippy::print_stdout)] // We cannot use `log` before we have the config file

pub mod file;
mod serde_bitcoin_network;
mod serde_duration;
mod settings;

pub use self::{
    file::{AllowedForeignOrigins, File},
    settings::Settings,
};
