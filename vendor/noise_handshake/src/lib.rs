#![warn(unused_extern_crates, rust_2018_idioms, missing_debug_implementations)]
#![deny(unsafe_code)]
#![feature(stmt_expr_attributes)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate debug_stub_derive;

pub mod handshake;
