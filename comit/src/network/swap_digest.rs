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
    #[digest(prefix = "99ef")]
    pub ethereum_expiry: Timestamp,
    #[digest(prefix = "f0ec")]
    pub erc20_amount: asset::Erc20Quantity,
    #[digest(prefix = "944a")]
    pub token_contract: identity::Ethereum,
    #[digest(prefix = "d07f")]
    pub lightning_cltv_expiry: RelativeTime,
    #[digest(prefix = "1475")]
    pub lightning_amount: asset::Bitcoin,
    #[digest(prefix = "41f4")]
    pub alpha_protocol: SwapProtocol,
    #[digest(prefix = "be74")]
    pub beta_protocol: SwapProtocol,
}
