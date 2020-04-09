use crate::swap_protocols::{NodeLocalSwapId, SwapId};
use pem::{encode, Pem};
use rand::Rng;
use sha2::{Digest, Sha256};
use std::{
    ffi::OsStr,
    fmt,
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
};
use thiserror;

/// We create a `RootSeed` either randomly or by reading in the PEM file from
/// disk.  This `RootSeed` is used to generate a per swap `SwapSeed` which is
/// then use as the secret source for deriving redeem/refund identities.
/// `RootSeed` and `SwapSeed` are the same underlying type (`Seed`), they exist
/// solely to allow the compiler to provide us with type safety.

// This will go away once rfc003 is gone.
#[ambassador::delegatable_trait]
pub trait DeriveSwapSeed {
    fn derive_swap_seed(&self, id: SwapId) -> SwapSeed;
}

impl DeriveSwapSeed for RootSeed {
    fn derive_swap_seed(&self, id: SwapId) -> SwapSeed {
        let data = self.sha256_with_seed(&[b"SWAP", id.0.as_bytes()]);
        SwapSeed(Seed(data))
    }
}

// This exists because its safer than the above trait now that the swap_id comes
// from Bob.  We do not want to derive the seed using information from Bob.
#[ambassador::delegatable_trait]
pub trait DeriveSwapSeedFromNodeLocal {
    fn derive_swap_seed_from_node_local(&self, id: NodeLocalSwapId) -> SwapSeed;
}

impl DeriveSwapSeedFromNodeLocal for RootSeed {
    fn derive_swap_seed_from_node_local(&self, id: NodeLocalSwapId) -> SwapSeed {
        let data = self.sha256_with_seed(&[b"SWAP", id.0.as_bytes()]);
        SwapSeed(Seed(data))
    }
}

const SEED_LENGTH: usize = 32;

#[derive(Clone, Copy, PartialEq)]
struct Seed([u8; SEED_LENGTH]);

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
    fn sha256_with_seed(&self, slices: &[&[u8]]) -> [u8; SEED_LENGTH] {
        let mut sha = Sha256::new();
        sha.input(&self.0);
        for slice in slices {
            sha.input(slice);
        }

        sha.result().into()
    }
}

impl From<[u8; SEED_LENGTH]> for Seed {
    fn from(seed: [u8; SEED_LENGTH]) -> Self {
        Seed(seed)
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct RootSeed(Seed);

impl fmt::Debug for RootSeed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl fmt::Display for RootSeed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct SwapSeed(Seed);

impl fmt::Debug for SwapSeed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl fmt::Display for SwapSeed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl RootSeed {
    pub fn sha256_with_seed(&self, slices: &[&[u8]]) -> [u8; SEED_LENGTH] {
        self.0.sha256_with_seed(slices)
    }

    pub fn new_random<R>(mut rand: R) -> Result<RootSeed, rand::Error>
    where
        R: Rng,
    {
        let mut arr = [0u8; SEED_LENGTH];
        rand.try_fill(&mut arr[..])?;
        Ok(RootSeed(Seed(arr)))
    }

    /// Read the seed from the default location if it exists, otherwise
    /// generate a random seed and write it to the default location.
    pub fn from_default_dir_or_generate<R>(rand: R) -> Result<RootSeed, Error>
    where
        R: Rng,
    {
        let path = default_seed_path()?;
        RootSeed::from_dir_or_generate(&path, rand)
    }

    /// Read the seed from the directory if it exists, otherwise
    /// generate a random seed and write it to that location.
    pub fn from_dir_or_generate<D, R>(data_dir: D, rand: R) -> Result<RootSeed, Error>
    where
        D: AsRef<OsStr>,
        R: Rng,
    {
        let dir = Path::new(&data_dir);
        let path = seed_path_from_dir(dir);

        if path.exists() {
            return Self::from_file(&path);
        }

        let random_seed = RootSeed::new_random(rand)?;
        random_seed.write_to(path.clone())?;

        tracing::info!("No seed file found, creating at: {}", path.display());

        Ok(random_seed)
    }

    fn from_file<D>(seed_file: D) -> Result<RootSeed, Error>
    where
        D: AsRef<OsStr>,
    {
        let file = Path::new(&seed_file);
        let contents = fs::read_to_string(file)?;
        let pem = pem::parse(contents)?;

        tracing::info!("Read in seed from file: {}", file.display());

        RootSeed::from_pem(pem)
    }

    fn from_pem(pem: pem::Pem) -> Result<RootSeed, Error> {
        if pem.contents.len() != SEED_LENGTH {
            Err(Error::IncorrectLength(pem.contents.len()))
        } else {
            let mut array = [0; SEED_LENGTH];
            for (i, b) in pem.contents.iter().enumerate() {
                array[i] = *b;
            }

            Ok(RootSeed::from(array))
        }
    }

    fn write_to(&self, seed_file: PathBuf) -> Result<(), Error> {
        ensure_directory_exists(seed_file.clone())?;
        self._write_to(seed_file)?;
        Ok(())
    }

    fn _write_to(&self, path: PathBuf) -> Result<(), Error> {
        let data = (self.0).0;
        let pem = Pem {
            tag: String::from("SEED"),
            contents: data.to_vec(),
        };

        let pem_string = encode(&pem);

        let mut file = File::create(path)?;
        file.write_all(pem_string.as_bytes())?;

        Ok(())
    }
}

impl SwapSeed {
    pub fn sha256_with_seed(&self, slices: &[&[u8]]) -> [u8; SEED_LENGTH] {
        self.0.sha256_with_seed(slices)
    }
}

fn ensure_directory_exists(file: PathBuf) -> Result<(), Error> {
    if let Some(path) = file.parent() {
        if !path.exists() {
            tracing::info!(
                "RootSeed file parent directory does not exist, creating recursively: {}",
                file.display()
            );
            fs::create_dir_all(path)?;
        }
    }
    Ok(())
}

fn default_seed_path() -> Result<PathBuf, Error> {
    let default_path = crate::data_dir().ok_or(Error::NoDefaultPath)?;
    Ok(seed_path_from_dir(&default_path))
}

fn seed_path_from_dir(dir: &Path) -> PathBuf {
    let path = dir.to_path_buf();
    path.join("seed.pem")
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io: ")]
    Io(#[from] io::Error),
    #[error("PEM parse: ")]
    PemParse(#[from] pem::PemError),
    #[error("expected 32 bytes of base64 encode, got {0} bytes")]
    IncorrectLength(usize),
    #[error("RNG: ")]
    Rand(#[from] rand::Error),
    #[error("no default path")]
    NoDefaultPath,
}

impl From<[u8; SEED_LENGTH]> for RootSeed {
    fn from(seed: [u8; SEED_LENGTH]) -> Self {
        RootSeed(Seed(seed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pem;
    use rand::rngs::OsRng;

    #[test]
    fn seed_byte_string_must_be_32_bytes_long() {
        let _seed = RootSeed::from(*b"this string is exactly 32 bytes!");
    }

    #[test]
    fn data_and_seed_used_to_calculate_hash() {
        let seed1 = RootSeed::from(*b"hello world, you are beautiful!!");
        assert_ne!(
            seed1.sha256_with_seed(&[b"foo"]),
            seed1.sha256_with_seed(&[b"bar"])
        );

        let seed2 = RootSeed::from(*b"bye world, you are beautiful!!!!");
        assert_ne!(
            seed1.sha256_with_seed(&[b"foo"]),
            seed2.sha256_with_seed(&[b"foo"])
        );
    }

    #[test]
    fn test_two_random_seeds_are_different() {
        let random1 = RootSeed::new_random(OsRng).unwrap();
        let random2 = RootSeed::new_random(OsRng).unwrap();

        assert_ne!(random1, random2);
    }

    #[test]
    fn test_display_and_debug_not_implemented() {
        let seed = RootSeed::new_random(OsRng).unwrap();

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
        let got = RootSeed::from_pem(pem).unwrap();

        assert_eq!((got.0).0, *want);
    }

    #[test]
    fn seed_from_pem_fails_for_short_seed() {
        let short = "-----BEGIN SEED-----
VnZUNFZ4dlY=
-----END SEED-----
";
        let pem = pem::parse(short).unwrap();
        match RootSeed::from_pem(pem) {
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
        match RootSeed::from_pem(pem) {
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

        let seed = RootSeed::new_random(OsRng).unwrap();
        seed._write_to(path.clone())
            .expect("Write seed to temp file");

        let rinsed = RootSeed::from_file(path).expect("Read from temp file");
        assert_eq!(seed.0, rinsed.0);
    }
}
