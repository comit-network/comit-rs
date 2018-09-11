extern crate regex;

use regex::Regex;
use std::{
    env::var,
    fs::File,
    io::Write,
    process::{Command, Stdio},
};

fn main() -> std::io::Result<()> {
    compile(include_str!("./ether_contract.asm"), "./ether_contract.asm")?;
    compile(
        include_str!("./ether_deploy_header.asm"),
        "./ether_deploy_header.asm",
    )
}

fn check_bin_in_path(bin: &str) {
    let output = Command::new("which").arg(bin).output().unwrap();
    if output.stdout.is_empty() {
        let mut msg = format!("`{}` cannot be found, check your path", bin);
        msg = format!("{}\nPATH: {:?}", msg, var("PATH"));
        panic!(msg);
    }
}

fn compile(code: &str, file_name: &'static str) -> std::io::Result<()> {
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

    solc.stdin.as_mut().unwrap().write_all(code.as_bytes())?;

    let output = solc.wait_with_output()?;
    let stdout = String::from_utf8(output.stdout).unwrap();
    let regex = Regex::new(r"\nBinary representation:\n(?P<hexcode>.+)\n").unwrap();

    let captures = regex
        .captures(stdout.as_str())
        .expect("Regex didn't match!");

    let hexcode = captures.name("hexcode").unwrap();

    let mut file = File::create(format!("{}.hex", file_name).as_str())?;
    file.write_all(hexcode.as_str().as_bytes())?;

    println!("cargo:rerun-if-changed={}", file_name);

    Ok(())
}
