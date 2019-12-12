use std::process::Command;
fn main() {
    let output = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .output()
        .expect("failed to get commit hash for tip of branch");

    let git_hash = String::from_utf8(output.stdout).expect("failed to convert hash");

    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
}
