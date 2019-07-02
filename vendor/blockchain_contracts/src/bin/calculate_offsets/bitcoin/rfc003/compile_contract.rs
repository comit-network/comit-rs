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
    let input = String::from_utf8(input)?;
    let input = input.replace("\n", " ").into_bytes();

    match bx.stdin {
        Some(ref mut stdin) => {
            stdin.write_all(&input)?;
            let output = bx.wait_with_output()?;
            let stdout = String::from_utf8(output.stdout)?;
            let bytes = hex::decode(stdout.trim())?;

            Ok(bytes)
        }
        None => Err(Error::CannotWriteInStdin),
    }
}
