extern crate regex;

use regex::Regex;
use std::{
	env::var,
	fs::{DirBuilder, File},
	io::Write,
	path::Path,
	process::{Command, Stdio},
};

fn main() -> std::io::Result<()> {
	compile("./src/swap_protocols/rfc003/ethereum/contract_templates/ether_contract.asm")?;
	compile("./src/swap_protocols/rfc003/ethereum/contract_templates/ether_deploy_header.asm")?;
	compile("./src/swap_protocols/rfc003/ethereum/contract_templates/erc20_contract.asm")?;
	compile("./src/swap_protocols/rfc003/ethereum/contract_templates/erc20_deploy_header.asm")?;

	Ok(())
}

fn compile(file_path: &'static str) -> std::io::Result<()> {
	let solc_bin = var("SOLC_BIN");

	let mut solc = match solc_bin {
		Ok(bin) => Command::new(bin)
			.arg("--assemble")
			.arg("-")
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

	let mut file = ::std::fs::File::open(file_path)?;

	::std::io::copy(&mut file, solc.stdin.as_mut().unwrap())?;

	let output = solc.wait_with_output()?;
	let stdout = String::from_utf8(output.stdout).unwrap();
	let regex = Regex::new(r"\nBinary representation:\n(?P<hexcode>.+)\n").unwrap();

	let captures = regex
		.captures(stdout.as_str())
		.expect("Regex didn't match!");

	let hexcode = captures.name("hexcode").unwrap();

	let path = Path::new(file_path);
	let folder = path.parent().unwrap().to_str().unwrap();
	let folder = format!("{}/out", folder);
	let file_name = path.file_name().unwrap().to_str().unwrap();

	DirBuilder::new().recursive(true).create(&folder).unwrap();

	let mut file = File::create(format!("{}/{}.hex", folder, file_name).as_str())?;
	file.write_all(hexcode.as_str().as_bytes())?;

	println!("cargo:rerun-if-changed={}", file_path);

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
