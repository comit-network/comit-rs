use crate::calculate_offsets::{bitcoin::rfc003::Error, check_bin_in_path};
use std::{
    ffi::OsStr,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

pub fn compile<S: AsRef<OsStr>>(file_path: S) -> Result<Vec<u8>, Error> {
    check_bin_in_path("docker");
    let mut bx = Command::new("docker")
        .arg("run")
        .arg("--rm")
        .arg("-i")
        .arg("coblox/libbitcoin-explorer:latest")
        .arg("script-encode")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    let input = std::fs::read(Path::new(&file_path))?;

    // Remove new lines in the input because `bx` only accept one-line inputs
    let regex = regex::bytes::Regex::new(r"\x0a")?;
    let input = regex.replace_all(&input, &b" "[..]);

    match bx.stdin {
        Some(ref mut stdin) => {
            stdin.write(&input)?;
            let output = bx.wait_with_output()?;
            let stdout = String::from_utf8(output.stdout).unwrap();
            let bytes = hex::decode(stdout.trim())?;

            Ok(bytes)
        }
        None => return Err(Error::CannotWriteInStdin),
    }
}
