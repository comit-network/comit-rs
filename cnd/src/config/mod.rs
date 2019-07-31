#![allow(clippy::print_stdout)] // We cannot use `log` before we have the config file

pub mod file;
mod serde_duration;
mod settings;

pub use self::{file::File, settings::Settings};
