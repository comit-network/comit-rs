#[macro_use]
extern crate rust_embed;

#[derive(RustEmbed)]
#[folder = "./vendor/comit_i/build/"]
pub struct Asset;
