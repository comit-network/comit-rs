use crate::{asset, ethereum, identity, RelativeTime, Timestamp};
use digest::{Digest, ToDigestInput};
use libp2p::multihash::Multihash;
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

pub fn herc20_halbit<S: Into<Herc20Halbit>>(swap: S) -> SwapDigest {
    swap.into().digest().into()
}

pub fn halbit_herc20<S: Into<HalbitHerc20>>(swap: S) -> SwapDigest {
    swap.into().digest().into()
}

pub fn herc20_hbit<S: Into<Herc20Hbit>>(swap: S) -> SwapDigest {
    swap.into().digest().into()
}

pub fn hbit_herc20<S: Into<HbitHerc20>>(swap: S) -> SwapDigest {
    swap.into().digest().into()
}

/// This represents the information that we use to create a swap digest for
/// herc20 <-> halbit swaps.
#[derive(Clone, Digest, Debug)]
#[digest(hash = "Sha3_256")]
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
    pub lightning_amount: Digestable<asset::Bitcoin>,
}

/// This represents the information that we use to create a swap digest for
/// halbit <-> herc20 swaps.
#[derive(Clone, Digest, Debug)]
#[digest(hash = "Sha3_256")]
pub struct HalbitHerc20 {
    #[digest(prefix = "2001")]
    pub lightning_cltv_expiry: RelativeTime,
    #[digest(prefix = "2002")]
    pub lightning_amount: Digestable<asset::Bitcoin>,
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
#[digest(hash = "Sha3_256")]
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
    pub bitcoin_amount: Digestable<asset::Bitcoin>,
}

/// This represents the information that we use to create a swap digest for
/// hbit <-> herc20 swaps.
#[derive(Clone, Digest, Debug, PartialEq)]
#[digest(hash = "Sha3_256")]
pub struct HbitHerc20 {
    #[digest(prefix = "2001")]
    pub bitcoin_expiry: Timestamp,
    #[digest(prefix = "2002")]
    pub bitcoin_amount: Digestable<asset::Bitcoin>,
    #[digest(prefix = "3001")]
    pub ethereum_expiry: Timestamp,
    #[digest(prefix = "3002")]
    pub erc20_amount: asset::Erc20Quantity,
    #[digest(prefix = "3003")]
    pub token_contract: identity::Ethereum,
}

/// A new-type that allows us to implemented `ToDigestInput` on foreign types.
#[derive(Clone, Debug, PartialEq)]
pub struct Digestable<T>(pub T);

#[derive(Clone, Debug, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct SwapDigest(Multihash);

#[cfg(test)]
impl SwapDigest {
    pub fn random() -> Self {
        use rand::{thread_rng, RngCore};

        let mut bytes = [0u8; 32];
        thread_rng().fill_bytes(&mut bytes);

        let hash = libp2p::multihash::Sha3_256::digest(&bytes);

        Self(hash)
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
        let multihash = Multihash::from_bytes(bytes).map_err(D::Error::custom)?;

        Ok(SwapDigest(multihash))
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Sha3_256(Multihash);

impl digest::Hash for Sha3_256 {
    fn hash(bytes: &[u8]) -> Self {
        Self(libp2p::multihash::Sha3_256::digest(bytes))
    }
}

impl ToDigestInput for Sha3_256 {
    fn to_digest_input(&self) -> Vec<u8> {
        self.0.to_vec()
    }
}

impl From<Sha3_256> for SwapDigest {
    fn from(sha3256: Sha3_256) -> Self {
        SwapDigest(sha3256.0)
    }
}

impl ToDigestInput for Timestamp {
    fn to_digest_input(&self) -> Vec<u8> {
        self.clone().to_bytes().to_vec()
    }
}

impl ToDigestInput for RelativeTime {
    fn to_digest_input(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }
}

impl ToDigestInput for ethereum::Address {
    fn to_digest_input(&self) -> Vec<u8> {
        self.clone().as_bytes().to_vec()
    }
}

impl ToDigestInput for Digestable<asset::Bitcoin> {
    fn to_digest_input(&self) -> Vec<u8> {
        self.0.as_sat().to_le_bytes().to_vec()
    }
}

impl ToDigestInput for asset::Ether {
    fn to_digest_input(&self) -> Vec<u8> {
        self.to_bytes()
    }
}

impl ToDigestInput for asset::Erc20Quantity {
    fn to_digest_input(&self) -> Vec<u8> {
        self.to_bytes()
    }
}
