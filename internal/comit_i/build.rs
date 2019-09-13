use std::{
    fs::File,
    io::{Read, Write},
};

fn main() {
    let version = include_str!("./version").trim();

    let url = format!(
        "https://github.com/comit-network/comit-i/releases/download/{}/bundle.zip",
        version
    );

    println!("Fetching {}", url);
    let mut tmpfile: File = tempfile::tempfile().expect("Could not create temp file");
    let mut buffer = Vec::new();
    ureq::get(&url)
        .call()
        .into_reader()
        .read_to_end(&mut buffer)
        .expect("Could not download bundle");
    tmpfile.write_all(&buffer).expect("Could not write bundle");
    unzip::Unzipper::new(tmpfile, "./")
        .unzip()
        .expect("Could not unzip bundle");

    println!("cargo:rerun-if-changed={}", "./version");
}
