use bitcoin_support::{self, BitcoinQuantity, OutPoint};
use bitcoin_witness;
use ethereum_support;
use secp256k1_support::KeyPair;
use swap_protocols::rfc003::{bitcoin, secret::Secret};

pub mod btc_eth;

enum Action<Fund, Redeem, Refund> {
    FundHtlc(Fund),
    RedeemHtlc(Redeem),
    RefundHtlc(Refund),
}

trait StateActions<Fund, Redeem, Refund> {
    fn actions(&self) -> Vec<Action<Fund, Redeem, Refund>>;
}

struct EtherDeploy {
    data: ethereum_support::Bytes,
    value: ethereum_support::EtherQuantity,
    gas_limit: u32,
}

struct EtherRefund {
    pub contract_address: ethereum_support::Address,
    pub execution_gas: u32,
}

struct BitcoinRedeem {
    pub outpoint: OutPoint,
    pub htlc: bitcoin::Htlc,
    pub value: BitcoinQuantity,
    pub transient_keypair: KeyPair,
    pub secret: Secret,
}

impl BitcoinRedeem {
    pub fn to_transaction(
        &self,
        to_address: bitcoin_support::Address,
    ) -> bitcoin_witness::PrimedTransaction {
        bitcoin_witness::PrimedTransaction {
            inputs: vec![bitcoin_witness::PrimedInput::new(
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
