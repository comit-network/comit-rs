use crate::{asset, identity, RelativeTime, Timestamp};
use digest::{Digest, ToDigestInput};
use libp2p::multihash::{self, Multihash};
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

pub fn herc20_halbit<S: Into<Herc20Halbit>>(swap: S) -> SwapDigest {
    swap.into().digest()
}

pub fn halbit_herc20<S: Into<HalbitHerc20>>(swap: S) -> SwapDigest {
    swap.into().digest()
}

pub fn herc20_hbit<S: Into<Herc20Hbit>>(swap: S) -> SwapDigest {
    swap.into().digest()
}

pub fn hbit_herc20<S: Into<HbitHerc20>>(swap: S) -> SwapDigest {
    swap.into().digest()
}

/// This represents the information that we use to create a swap digest for
/// herc20 <-> halbit swaps.
#[derive(Clone, Digest, Debug)]
#[digest(hash = "SwapDigest")]
pub struct Herc20Halbit {
    #[digest(prefix = "2001")]
    pub ethereum_absolute_expiry: Timestamp,
    #[digest(prefix = "2002")]
    pub erc20_amount: asset::Erc20Quantity,
    #[digest(prefix = "2003")]
    pub token_contract: identity::Ethereum,
    #[digest(prefix = "3001")]
    pub lightning_cltv_expiry: RelativeTime,
    #[digest(prefix = "3002")]
    pub lightning_amount: asset::Bitcoin,
}

/// This represents the information that we use to create a swap digest for
/// halbit <-> herc20 swaps.
#[derive(Clone, Digest, Debug)]
#[digest(hash = "SwapDigest")]
pub struct HalbitHerc20 {
    #[digest(prefix = "2001")]
    pub lightning_cltv_expiry: RelativeTime,
    #[digest(prefix = "2002")]
    pub lightning_amount: asset::Bitcoin,
    #[digest(prefix = "3001")]
    pub ethereum_absolute_expiry: Timestamp,
    #[digest(prefix = "3002")]
    pub erc20_amount: asset::Erc20Quantity,
    #[digest(prefix = "3003")]
    pub token_contract: identity::Ethereum,
}

/// This represents the information that we use to create a swap digest for
/// herc20 <-> hbit swaps.
#[derive(Clone, Digest, Debug, PartialEq)]
#[digest(hash = "SwapDigest")]
pub struct Herc20Hbit {
    #[digest(prefix = "2001")]
    pub ethereum_expiry: Timestamp,
    #[digest(prefix = "2002")]
    pub erc20_amount: asset::Erc20Quantity,
    #[digest(prefix = "2003")]
    pub token_contract: identity::Ethereum,
    #[digest(prefix = "3001")]
    pub bitcoin_expiry: Timestamp,
    #[digest(prefix = "3002")]
    pub bitcoin_amount: asset::Bitcoin,
}

/// This represents the information that we use to create a swap digest for
/// hbit <-> herc20 swaps.
#[derive(Clone, Digest, Debug, PartialEq)]
#[digest(hash = "SwapDigest")]
pub struct HbitHerc20 {
    #[digest(prefix = "2001")]
    pub bitcoin_expiry: Timestamp,
    #[digest(prefix = "2002")]
    pub bitcoin_amount: asset::Bitcoin,
    #[digest(prefix = "3001")]
    pub ethereum_expiry: Timestamp,
    #[digest(prefix = "3002")]
    pub erc20_amount: asset::Erc20Quantity,
    #[digest(prefix = "3003")]
    pub token_contract: identity::Ethereum,
}

#[derive(Clone, Debug, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct SwapDigest(Multihash);

impl SwapDigest {
    pub fn new(multihash: Multihash) -> Self {
        Self(multihash)
    }
}

impl ToDigestInput for SwapDigest {
    fn to_digest_input(&self) -> Vec<u8> {
        self.0.clone().into_bytes()
    }
}

impl digest::Hash for SwapDigest {
    fn hash(bytes: &[u8]) -> Self {
        Self(multihash::Sha3_256::digest(bytes))
    }
}

impl fmt::Display for SwapDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0.as_bytes()))
    }
}

impl Serialize for SwapDigest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let hex = hex::encode(self.0.as_bytes());

        serializer.serialize_str(&hex)
    }
}

impl<'de> Deserialize<'de> for SwapDigest {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        let hex = String::deserialize(deserializer)?;
        let bytes = hex::decode(hex).map_err(D::Error::custom)?;
        let multihash = multihash::Multihash::from_bytes(bytes).map_err(D::Error::custom)?;

        Ok(SwapDigest::new(multihash))
    }
}
