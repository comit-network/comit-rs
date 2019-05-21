use crate::calculate_offsets::ethereum::rfc003::Error::{self, CaptureSolcBytecode};
use regex::Regex;
use std::{
    env::var,
    ffi::OsStr,
    process::{Command, Stdio},
};

pub fn compile<S: AsRef<OsStr>>(file_path: S) -> Result<Vec<u8>, Error> {
    let solc_bin = var("SOLC_BIN");

    let mut solc = match solc_bin {
        Ok(bin) => Command::new(bin)
            .arg("--assemble")
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
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
                .stderr(Stdio::null())
                .spawn()?
        }
    };

    let mut file = ::std::fs::File::open(OsStr::new(&file_path))?;

    ::std::io::copy(&mut file, solc.stdin.as_mut().unwrap())?;

    let output = solc.wait_with_output()?;
    let stdout = String::from_utf8(output.stdout).unwrap();
    let regex = Regex::new(r"\nBinary representation:\n(?P<hexcode>.+)\n")?;

    let captures = regex
        .captures(stdout.as_str())
        .expect("Regex didn't match!");

    let hexcode = captures.name("hexcode").ok_or(CaptureSolcBytecode)?;
    let bytes = hex::decode(hexcode.as_str())?;

    Ok(bytes)
}

fn check_bin_in_path(bin: &str) {
    let output = Command::new("which").arg(bin).output().unwrap();
    if output.stdout.is_empty() {
        let mut msg = format!("`{}` cannot be found, check your path", bin);
        msg = format!("{}\nPATH: {:?}", msg, var("PATH"));
        panic!(msg);
    }
}
