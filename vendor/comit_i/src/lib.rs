#[macro_use]
extern crate rust_embed;

use http::uri::PathAndQuery;
use std::str::FromStr;

#[derive(RustEmbed)]
#[folder = "./vendor/comit_i/build/"]
pub struct Asset;
