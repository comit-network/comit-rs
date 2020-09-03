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
    use crate::{asset, ledger};
    use bitcoin::{
        secp256k1::{self, Secp256k1},
        OutPoint,
    };
    use blockchain_contracts::bitcoin::witness::PrimedInput;

    pub use bitcoin::{Address, Amount, Transaction};
    pub use blockchain_contracts::bitcoin::witness::{PrimedTransaction, UnlockParameters};

    #[derive(Debug, Clone, PartialEq)]
    pub struct SendToAddress {
        pub to: Address,
        pub amount: asset::Bitcoin,
        pub network: ledger::Bitcoin,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct SpendOutput {
        // Remember: One man's input is another man's output!
        pub output: PrimedInput,
        pub network: ledger::Bitcoin,
    }

    impl SpendOutput {
        pub fn new(
            previous_output: OutPoint,
            value: Amount,
            input_parameters: UnlockParameters,
            network: ledger::Bitcoin,
        ) -> Self {
            Self {
                output: PrimedInput::new(previous_output, value, input_parameters),
                network,
            }
        }

        pub fn spend_to(self, to_address: Address) -> PrimedTransaction {
            PrimedTransaction {
                inputs: vec![self.output],
                output_address: to_address,
            }
        }
    }

    pub fn sign_with_fixed_rate<C>(
        secp: &Secp256k1<C>,
        primed_transaction: PrimedTransaction,
    ) -> anyhow::Result<Transaction>
    where
        C: secp256k1::Signing,
    {
        let rate = 10;
        primed_transaction
            .sign_with_rate(secp, rate)
            .map_err(|_| anyhow::anyhow!("failed to sign with {} rate", rate))
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct BroadcastSignedTransaction {
        pub transaction: Transaction,
        pub network: ledger::Bitcoin,
    }
}

pub mod ethereum {
    use crate::{asset, ethereum::ChainId, identity, Timestamp};

    #[derive(Debug, Clone, PartialEq)]
    pub struct DeployContract {
        pub data: Vec<u8>,
        pub amount: asset::Ether,
        pub gas_limit: u64,
        pub chain_id: ChainId,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct CallContract {
        pub to: identity::Ethereum,
        pub data: Option<Vec<u8>>,
        pub gas_limit: u64,
        pub chain_id: ChainId,
        pub min_block_timestamp: Option<Timestamp>,
    }
}

pub mod lnd {
    use crate::{asset, identity, ledger, timestamp::RelativeTime, Secret, SecretHash};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct AddHoldInvoice {
        pub amount: asset::Bitcoin,
        pub secret_hash: SecretHash,
        pub expiry: RelativeTime, // The invoice's expiry
        pub cltv_expiry: RelativeTime,
        pub chain: Chain,
        pub network: ledger::Bitcoin,
        pub self_public_key: identity::Lightning,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct SettleInvoice {
        pub secret: Secret,
        pub chain: Chain,
        pub network: ledger::Bitcoin,
        pub self_public_key: identity::Lightning,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct CancelInvoice {
        pub secret_hash: SecretHash, // The hash of the preimage used when adding the invoice.
        pub chain: Chain,
        pub network: ledger::Bitcoin,
        pub self_public_key: identity::Lightning,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct SendPayment {
        pub to_public_key: identity::Lightning,
        pub amount: asset::Bitcoin,  // The number of satoshis to send.
        pub secret_hash: SecretHash, // The hash to use within the payment's HTLC.
        pub final_cltv_delta: RelativeTime,
        pub chain: Chain,
        pub network: ledger::Bitcoin,
        pub self_public_key: identity::Lightning,
    }

    /// The underlying chain i.e., layer 1, targeted by LND.
    #[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum Chain {
        Bitcoin,
    }
}
