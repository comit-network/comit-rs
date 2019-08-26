use crate::calculate_offsets::bitcoin::rfc003::Error;
use std::{path::Path, str::FromStr};

pub fn compile<S: AsRef<Path>>(file_path: S) -> Result<Vec<u8>, Error> {
    let input = std::fs::read_to_string(file_path)?.replace("\n", "");

    let policy = miniscript::policy::Concrete::<bitcoin::PublicKey>::from_str(&input)?;
    let miniscript = policy.compile();
    let descriptor = miniscript::Descriptor::Wsh(miniscript);

    let script = descriptor.witness_script();

    Ok(script.into_bytes())
}
