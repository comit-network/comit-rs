use std::fs::File;

fn main() {
    let version = include_str!("./version").trim();

    let url = format!(
        "https://github.com/comit-network/comit-i/releases/download/{}/bundle.zip",
        version
    );

    println!("Fetching {}", url);
    let mut tmpfile: File = tempfile::tempfile().expect("Could not create temp file");
    reqwest::get(&url)
        .unwrap()
        .copy_to(&mut tmpfile)
        .expect("Could not download bundle");
    unzip::Unzipper::new(tmpfile, "./")
        .unzip()
        .expect("Could not unzip bundle");

    println!("cargo:rerun-if-changed={}", "./version");
}
