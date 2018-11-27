use bitcoin_support::{Address, BitcoinQuantity, OutPoint};
use bitcoin_witness::{PrimedInput, PrimedTransaction};
use secp256k1_support::KeyPair;
use swap_protocols::rfc003::{bitcoin::Htlc, Secret};

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Serialize)]
pub struct BitcoinFund {
    pub address: Address,
    pub value: BitcoinQuantity,
}

impl BitcoinFund {
    pub fn new(address: Address, value: BitcoinQuantity) -> BitcoinFund {
        BitcoinFund { address, value }
    }
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct BitcoinRefund {
    pub outpoint: OutPoint,
    pub htlc: Htlc,
    pub value: BitcoinQuantity,
    pub transient_keypair: KeyPair,
}

impl BitcoinRefund {
    pub fn new(
        outpoint: OutPoint,
        htlc: Htlc,
        value: BitcoinQuantity,
        transient_keypair: KeyPair,
    ) -> BitcoinRefund {
        BitcoinRefund {
            outpoint,
            htlc,
            value,
            transient_keypair,
        }
    }

    pub fn to_transaction(&self, to_address: Address) -> PrimedTransaction {
        PrimedTransaction {
            inputs: vec![PrimedInput::new(
                self.outpoint,
                self.value,
                self.htlc.unlock_after_timeout(self.transient_keypair),
            )],
            locktime: 0,
            output_address: to_address,
        }
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
    pub fn new(
        outpoint: OutPoint,
        htlc: Htlc,
        value: BitcoinQuantity,
        transient_keypair: KeyPair,
        secret: Secret,
    ) -> BitcoinRedeem {
        BitcoinRedeem {
            outpoint,
            htlc,
            value,
            transient_keypair,
            secret,
        }
    }

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
}
