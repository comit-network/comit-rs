/// Common interface across all protocols supported by COMIT
///
/// This trait is intended to be implemented on an Actor's state and return the
/// actions which are currently available in a given state.
pub trait Actions {
    /// Different protocols have different kinds of requirements for actions.
    /// Hence they get to choose the type here.
    type ActionKind;

    fn actions(&self) -> Vec<Self::ActionKind>;
}

pub mod bitcoin {
    use crate::swap_protocols::rfc003::Secret;
    use bitcoin_support::{Address, BitcoinQuantity, Network, OutPoint};
    use blockchain_contracts::bitcoin::rfc003::bitcoin_htlc::BitcoinHtlc;
    use secp256k1_keypair::SecretKey;
    use serde::Serialize;

    #[derive(Debug, Clone, PartialEq, Serialize)]
    pub struct SendToAddress {
        pub to: Address,
        pub amount: BitcoinQuantity,
        pub network: Network,
    }

    #[derive(Debug)]
    pub struct SpendHtlc {
        pub network: Network,
        pub outpoint: OutPoint,
        pub amount: BitcoinQuantity,
        pub key: SecretKey,
        pub secret: Option<Secret>,
        pub htlc: BitcoinHtlc,
    }
}

pub mod ethereum {
    use crate::swap_protocols::Timestamp;
    use ethereum_support::{web3::types::U256, Address, Bytes, EtherQuantity, Network};
    use serde::Serialize;

    #[derive(Debug, Clone, PartialEq, Serialize)]
    pub struct DeployContract {
        pub data: Bytes,
        pub amount: EtherQuantity,
        pub gas_limit: U256,
        pub network: Network,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct CallContract {
        pub to: Address,
        pub data: Option<Bytes>,
        pub gas_limit: U256,
        pub network: Network,
        pub min_block_timestamp: Option<Timestamp>,
    }
}
