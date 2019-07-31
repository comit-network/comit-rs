#![allow(clippy::print_stdout)] // We cannot use `log` before we have the config file

mod config_file;
mod serde_duration;
mod settings;

pub use self::{
    config_file::{read_from, read_or_create_default, ConfigFile},
    settings::Settings,
};
