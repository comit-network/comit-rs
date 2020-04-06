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
    use crate::asset;
    use bitcoin::Address;
    use blockchain_contracts::bitcoin::witness::{PrimedInput, PrimedTransaction};

    #[derive(Debug, Clone, PartialEq)]
    pub struct SendToAddress {
        pub to: Address,
        pub amount: asset::Bitcoin,
        pub network: bitcoin::Network,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct SpendOutput {
        // Remember: One man's input is another man's output!
        pub output: PrimedInput,
        pub network: bitcoin::Network,
    }

    impl SpendOutput {
        pub fn spend_to(self, to_address: Address) -> PrimedTransaction {
            PrimedTransaction {
                inputs: vec![self.output],
                output_address: to_address,
            }
        }
    }
}

pub mod ethereum {
    use crate::{
        asset, ethereum::Bytes, identity, swap_protocols::ledger::ethereum::ChainId,
        timestamp::Timestamp,
    };

    #[derive(Debug, Clone, PartialEq)]
    pub struct DeployContract {
        pub data: Bytes,
        pub amount: asset::Ether,
        pub gas_limit: u64,
        pub chain_id: ChainId,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct CallContract {
        pub to: identity::Ethereum,
        pub data: Option<Bytes>,
        pub gas_limit: u64,
        pub chain_id: ChainId,
        pub min_block_timestamp: Option<Timestamp>,
    }
}

pub mod lnd {
    use crate::{
        asset, identity,
        swap_protocols::rfc003::{Secret, SecretHash},
    };

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct AddHoldInvoice {
        pub amount: asset::Lightning, // The number of satoshis to send.
        pub secret_hash: SecretHash,  // The hash to use within the payment's HTLC.
        pub expiry: u32,
        pub cltv_expiry: u32,
        pub chain: Chain,
        pub network: bitcoin::Network,
        pub self_public_key: identity::Lightning,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct SettleInvoice {
        pub secret: Secret,
        pub chain: Chain,
        pub network: bitcoin::Network,
        pub self_public_key: identity::Lightning,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct CancelInvoice {
        pub secret_hash: SecretHash, // The hash of the preimage used when adding the invoice.
        pub chain: Chain,
        pub network: bitcoin::Network,
        pub self_public_key: identity::Lightning,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct SendPayment {
        pub to_public_key: identity::Lightning,
        pub amount: asset::Lightning, // The number of satoshis to send.
        pub secret_hash: SecretHash,  // The hash to use within the payment's HTLC.
        pub final_cltv_delta: u32,
        pub chain: Chain,
        pub network: bitcoin::Network,
        pub self_public_key: identity::Lightning,
    }

    /// The underlying chain i.e., layer 1, targeted by LND.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum Chain {
        Bitcoin,
    }
}
