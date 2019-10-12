extern crate vergen;

use vergen::{generate_cargo_keys, ConstantsFlags};

fn main() {
    generate_cargo_keys(ConstantsFlags::all()).expect("Unable to generate the cargo keys!");
}
