use crate::{asset, ethereum::ChainId, identity, ledger};
use anyhow::{Context, Result};
use bitcoin::{
    secp256k1::{self, Secp256k1},
    Address, Amount, OutPoint, Transaction,
};
use blockchain_contracts::bitcoin::witness::{PrimedInput, PrimedTransaction, UnlockParameters};

#[derive(Debug, Clone, PartialEq)]
pub struct SendToAddress {
    /// Where the Bitcoins should be sent to.
    pub to: Address,
    /// How many Bitcoins should be sent.
    pub amount: asset::Bitcoin,
    /// The network this should happen on.
    /// TODO: This is redundant with the address.
    pub network: ledger::Bitcoin,
}

#[derive(Debug, Clone)]
pub struct SpendOutput {
    pub tx: PrimedTransaction,
    pub network: ledger::Bitcoin,
}

impl SpendOutput {
    pub fn new(
        previous_output: OutPoint,
        value: Amount,
        input_parameters: UnlockParameters,
        network: ledger::Bitcoin,
        to_address: Address,
    ) -> Self {
        let output = PrimedInput::new(previous_output, value, input_parameters);
        Self {
            tx: PrimedTransaction {
                inputs: vec![output],
                output_address: to_address,
            },
            network,
        }
    }

    pub fn sign<C>(self, secp: &Secp256k1<C>, byte_rate: bitcoin::Amount) -> Result<Transaction>
    where
        C: secp256k1::Signing,
    {
        let transaction = self
            .tx
            .sign_with_rate(secp, byte_rate)
            .with_context(|| format!("failed to sign with {} rate", byte_rate))?;

        Ok(transaction)
    }
}

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
}
