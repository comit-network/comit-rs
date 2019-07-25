#![warn(unused_extern_crates, rust_2018_idioms)]
#![forbid(unsafe_code)]

#[macro_use]
extern crate rust_embed;

#[derive(RustEmbed)]
#[folder = "./vendor/comit_i/build/"]
pub struct Asset;
