use crate::{asset, identity, network::protocols::announce::SwapDigest, RelativeTime, Timestamp};
use digest::{Digest, ToDigestInput};

// TODO should we use swap_protocols::SwapProtocol instead?
#[derive(Clone, Digest, Debug, PartialEq, Copy)]
#[digest(hash = "SwapDigest")]
pub enum SwapProtocol {
    #[digest(prefix = "4b17")]
    Hbit,
    #[digest(prefix = "e5ec")]
    Herc20,
    #[digest(prefix = "c3d3")]
    Halight,
}

// TODO why is this necessary?
impl ToDigestInput for SwapProtocol {
    fn to_digest_input(&self) -> Vec<u8> {
        self.clone().digest().to_digest_input()
    }
}

/// This represents the information that we use to create a swap digest for
/// herc20 <-> halight swaps.
#[derive(Clone, Digest, Debug)]
#[digest(hash = "SwapDigest")]
pub struct Herc20Halight {
    #[digest(prefix = "2001")]
    pub ethereum_expiry: Timestamp,
    #[digest(prefix = "2002")]
    pub erc20_amount: asset::Erc20Quantity,
    #[digest(prefix = "2003")]
    pub token_contract: identity::Ethereum,
    #[digest(prefix = "3001")]
    pub lightning_cltv_expiry: RelativeTime,
    #[digest(prefix = "3002")]
    pub lightning_amount: asset::Bitcoin,
    #[digest(ignore)]
    pub alpha_protocol: SwapProtocol,
    #[digest(ignore)]
    pub beta_protocol: SwapProtocol,
}
