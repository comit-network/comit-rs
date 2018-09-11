extern crate regex;

use regex::Regex;
use std::{
    env::var,
    fs::File,
    io::Write,
    process::{Command, Stdio},
};

const CONTRACT: &str = include_str!("./ether_contract.asm");

fn main() -> std::io::Result<()> {
    let solc_bin = var("SOLC_BIN");

    let mut solc = match solc_bin {
        Ok(bin) => Command::new(bin)
            .arg("--assemble")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?,
        Err(_) => {
            check_bin_in_path("docker");
            Command::new("docker")
                .arg("run")
                .arg("--rm")
                .arg("-i")
                .arg("ethereum/solc:0.4.24")
                .arg("--assemble")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()?
        }
    };

    solc.stdin.as_mut().unwrap().write_all(CONTRACT.as_bytes())?;

    let output = solc.wait_with_output()?;
    let stdout = String::from_utf8(output.stdout).unwrap();
    let regex = Regex::new(r"\nBinary representation:\n(?P<hexcode>.+)\n").unwrap();

    let captures = regex
        .captures(stdout.as_str())
        .expect("Regex didn't match!");

    let hexcode = captures.name("hexcode").unwrap();

    let mut file = File::create("ether_contract.asm.hex")?;
    file.write_all(hexcode.as_str().as_bytes())?;

    println!("cargo:rerun-if-changed=./ether_contract.asm");

    Ok(())
}

fn check_bin_in_path(bin: &str) {
    let output = Command::new("which").arg(bin).output().unwrap();
    if output.stdout.is_empty() {
        let mut msg = format!("`{}` cannot be found, check your path", bin);
        msg = format!("{}\nPATH: {:?}", msg, var("PATH"));
        panic!(msg);
    }
}
