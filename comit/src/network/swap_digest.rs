use crate::{asset, identity, network::protocols::announce::SwapDigest, RelativeTime, Timestamp};
use digest::Digest;

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
