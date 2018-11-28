use bitcoin_support::{Address, BitcoinQuantity, OutPoint};
use bitcoin_witness::{PrimedInput, PrimedTransaction};
use secp256k1_support::KeyPair;
use swap_protocols::rfc003::{bitcoin::Htlc, Secret};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SendToAddress {
    pub address: Address,
    pub value: BitcoinQuantity,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpendOutput {
    //Remember: One man's input is another man's output!
    //TODO: decide whether we want to serialize this directly
    pub output: PrimedInput,
}

impl SpendOutput {
    pub fn spend_to(self, to_address: Address) -> PrimedTransaction {
        PrimedTransaction {
            inputs: vec![self.output],
            locktime: 0,
            output_address: to_address,
        }
    }
}

impl SpendOutput {
    pub fn serialize(&self, to: String) -> Result<String, ()> {
        unimplemented!()
    }
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct BitcoinRedeem {
    pub outpoint: OutPoint,
    pub htlc: Htlc,
    pub value: BitcoinQuantity,
    pub transient_keypair: KeyPair,
    pub secret: Secret,
}

impl BitcoinRedeem {
    pub fn to_transaction(&self, to_address: Address) -> PrimedTransaction {
        PrimedTransaction {
            inputs: vec![PrimedInput::new(
                self.outpoint,
                self.value,
                self.htlc
                    .unlock_with_secret(self.transient_keypair, &self.secret),
            )],
            locktime: 0,
            output_address: to_address,
        }
    }

    pub fn serialize(&self, to: String) -> Result<String, ()> {
        unimplemented!()
    }
}
