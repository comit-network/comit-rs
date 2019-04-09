use std::fs::File;

fn main() {
    let version = include_str!("./version").trim();

    let url = format!(
        "https://github.com/comit-network/comit-i/releases/download/{}/bundle.zip",
        version
    );

    let mut tmpfile: File = tempfile::tempfile().unwrap();
    reqwest::get(&url).unwrap().copy_to(&mut tmpfile).unwrap();
    unzip::Unzipper::new(tmpfile, "./").unzip().unwrap();

    println!("cargo:rerun-if-changed={}", "./version");
}
