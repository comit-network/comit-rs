use std::process::Command;
fn main() {
    let unknown = String::from("************");

    let output = Command::new("git").args(&["rev-parse", "HEAD"]).output();

    let git_hash = match output {
        Ok(output) => String::from_utf8(output.stdout).unwrap_or(unknown),
        Err(_) => unknown,
    };

    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
}
