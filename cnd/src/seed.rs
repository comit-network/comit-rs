use crate::{std_ext::path::PrintablePath, swap_protocols::SwapId};
use crypto::{digest::Digest, sha2::Sha256};
use pem::{encode, Pem};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    fmt,
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
};

pub const SEED_LENGTH: usize = 32;
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Seed(#[serde(with = "hex_serde")] [u8; SEED_LENGTH]);

impl fmt::Debug for Seed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Seed([*****])")
    }
}

impl fmt::Display for Seed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Seed {
    pub fn swap_seed(&self, id: SwapId) -> Seed {
        Seed(self.sha256_with_seed(&[b"SWAP", id.0.as_bytes()]))
    }
    pub fn sha256_with_seed(&self, slices: &[&[u8]]) -> [u8; SEED_LENGTH] {
        let mut sha = Sha256::new();
        sha.input(&self.0);
        for slice in slices {
            sha.input(slice);
        }
        let mut result = [0u8; SEED_LENGTH];
        sha.result(&mut result);
        result
    }

    pub fn new_random<R: Rng>(mut rand: R) -> Result<Seed, rand::Error> {
        let mut arr = [0u8; 32];
        rand.try_fill(&mut arr[..])?;
        Ok(Seed(arr))
    }

    /// Construct a seed from base64 data read in from pem file.
    pub fn from_file<D: AsRef<OsStr>>(seed_file: D) -> Result<Seed, Error> {
        let file = Path::new(&seed_file);
        log::info!(
            "Found seed file, reading from {}",
            PrintablePath(&file.to_path_buf())
        );

        let contents = fs::read_to_string(file)?;
        let pem = pem::parse(contents)?;

        Seed::from_pem(pem)
    }

    fn from_pem(pem: pem::Pem) -> Result<Seed, Error> {
        if pem.contents.len() != SEED_LENGTH {
            Err(Error::IncorrectLength(pem.contents.len()))
        } else {
            let mut array = [0; SEED_LENGTH];
            for (i, b) in pem.contents.iter().enumerate() {
                array[i] = *b;
            }

            Ok(Seed::from(array))
        }
    }

    /// Read the seed from the default location if it exists, otherwise
    /// generate a random seed and write it to the default location.
    pub fn from_default_file_or_generate<R: Rng>(rand: R) -> Result<Seed, Error> {
        let path = default_seed_path()?;

        if path.exists() {
            return Self::from_file(&path);
        }

        let random_seed = Seed::new_random(rand)?;
        random_seed.write_to(path.clone())?;

        log::info!(
            "No seed file found, creating default at {}",
            PrintablePath(&path)
        );

        Ok(random_seed)
    }

    fn write_to(&self, seed_file: PathBuf) -> Result<(), Error> {
        ensure_directory_exists(seed_file.clone())?;
        self._write_to(seed_file)?;
        Ok(())
    }

    fn _write_to(&self, path: PathBuf) -> Result<(), Error> {
        let pem = Pem {
            tag: String::from("SEED"),
            contents: self.0.to_vec(),
        };

        let pem_string = encode(&pem);

        let mut file = File::create(path.clone())?;
        file.write_all(pem_string.as_bytes())?;

        Ok(())
    }
}

pub trait SwapSeed {
    fn swap_seed(&self, id: SwapId) -> Seed;
}

impl SwapSeed for Seed {
    fn swap_seed(&self, id: SwapId) -> Seed {
        self.swap_seed(id)
    }
}

fn ensure_directory_exists(file: PathBuf) -> Result<(), Error> {
    if let Some(path) = file.parent() {
        if !path.exists() {
            log::info!(
                "Seed file parent directory does not exist, creating recursively: {}",
                PrintablePath(&file)
            );
            fs::create_dir_all(path)?;
        }
    }
    Ok(())
}

fn default_seed_path() -> Result<PathBuf, Error> {
    crate::data_dir()
        .map(|dir| Path::join(&dir, "seed.pem"))
        .ok_or(Error::NoDefaultPath)
}

pub enum Error {
    Io(io::Error),
    PemParse(pem::PemError),
    IncorrectLength(usize),
    Rand(rand::Error),
    NoDefaultPath,
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "seed file error")
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "seed: ")?;
        match self {
            Error::Io(e) => write!(f, "io error: {:?}", e),
            Error::PemParse(e) => write!(f, "pem format incorrect: {:?}", e),
            Error::IncorrectLength(x) => {
                write!(f, "expected 32 bytes of base64 encode, got {} bytes", x)
            }
            Error::Rand(e) => write!(f, "random number error: {:?}", e),
            Error::NoDefaultPath => write!(f, "failed to generate default path"),
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::Io(e)
    }
}

impl From<pem::PemError> for Error {
    fn from(e: pem::PemError) -> Error {
        Error::PemParse(e)
    }
}

impl From<rand::Error> for Error {
    fn from(e: rand::Error) -> Error {
        Error::Rand(e)
    }
}

impl From<[u8; 32]> for Seed {
    fn from(seed: [u8; 32]) -> Self {
        Seed(seed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pem;
    use rand::rngs::OsRng;

    #[test]
    fn seed_byte_string_must_be_32_bytes_long() {
        let _seed = Seed::from(*b"this string is exactly 32 bytes!");
    }

    #[test]
    fn data_and_seed_used_to_calculate_hash() {
        let seed1 = Seed::from(*b"hello world, you are beautiful!!");
        assert_ne!(
            seed1.sha256_with_seed(&[b"foo"]),
            seed1.sha256_with_seed(&[b"bar"])
        );

        let seed2 = Seed::from(*b"bye world, you are beautiful!!!!");
        assert_ne!(
            seed1.sha256_with_seed(&[b"foo"]),
            seed2.sha256_with_seed(&[b"foo"])
        );
    }

    #[test]
    fn test_two_random_seeds_are_different() {
        let random1 = Seed::new_random(OsRng).unwrap();
        let random2 = Seed::new_random(OsRng).unwrap();

        assert_ne!(random1, random2);
    }

    #[test]
    fn test_display_and_debug_not_implemented() {
        let seed = Seed::new_random(OsRng).unwrap();

        let out = seed.to_string();
        assert_eq!(out, "Seed([*****])".to_string());
        let debug = format!("{:?}", seed);
        assert_eq!(debug, "Seed([*****])".to_string());
    }

    #[test]
    fn seed_from_pem_works() {
        let payload: &str = "syl9wSYaruvgxg9P5Q1qkZaq5YkM6GvXkxe+VYrL/XM=";

        // 32 bytes base64 encoded.
        let pem_string: &str = "-----BEGIN SEED-----
syl9wSYaruvgxg9P5Q1qkZaq5YkM6GvXkxe+VYrL/XM=
-----END SEED-----
";

        let want = base64::decode(payload).unwrap();
        let pem = pem::parse(pem_string).unwrap();
        let got = Seed::from_pem(pem).unwrap();

        assert_eq!(got.0, *want);
    }

    #[test]
    fn seed_from_pem_fails_for_short_seed() {
        let short = "-----BEGIN SEED-----
VnZUNFZ4dlY=
-----END SEED-----
";
        let pem = pem::parse(short).unwrap();
        match Seed::from_pem(pem) {
            Ok(_) => panic!("should fail for short payload"),
            Err(e) => {
                match e {
                    Error::IncorrectLength(_) => {} // pass
                    _ => panic!("should fail with IncorrectLength error"),
                }
            }
        }
    }

    #[test]
    #[should_panic]
    fn seed_from_pem_fails_for_long_seed() {
        let long = "-----BEGIN SEED-----
mbKANv2qKGmNVg1qtquj6Hx1pFPelpqOfE2JaJJAMEg1FlFhNRNlFlE=
mbKANv2qKGmNVg1qtquj6Hx1pFPelpqOfE2JaJJAMEg1FlFhNRNlFlE=
-----END SEED-----
";
        let pem = pem::parse(long).unwrap();
        match Seed::from_pem(pem) {
            Ok(_) => panic!("should fail for short payload"),
            Err(e) => {
                match e {
                    Error::IncorrectLength(_) => {} // pass
                    _ => panic!("should fail with IncorrectLength error"),
                }
            }
        }
    }

    #[test]
    fn round_trip_through_file_write_read() {
        let tmpfile = tempfile::NamedTempFile::new().expect("Could not create temp file");
        let path = tmpfile.path().to_path_buf();

        let seed = Seed::new_random(OsRng).unwrap();
        seed._write_to(path.clone())
            .expect("Write seed to temp file");

        let rinsed = Seed::from_file(path).expect("Read from temp file");
        assert_eq!(seed.0, rinsed.0);
    }
}
