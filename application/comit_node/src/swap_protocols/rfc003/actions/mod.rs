use bitcoin_support::{self, BitcoinQuantity};
use bitcoin_witness;
use ethereum_support;
use secp256k1_support::KeyPair;
use swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::{bitcoin, secret::Secret},
};

enum Action<Fund, Redeem, Refund> {
    FundHtlc(Fund),
    RedeemHtlc(Redeem),
    RefundHtlc(Refund),
}

trait StateActions<Fund, Redeem, Refund> {
    fn actions(&self) -> Vec<Action<Fund, Redeem, Refund>>;
}

struct BitcoinFund {
    pub address: bitcoin_support::Address,
    pub value: BitcoinQuantity,
}

struct BitcoinRefund {
    pub htlc_id: bitcoin::HtlcId,
    pub htlc: bitcoin::Htlc,
    pub value: BitcoinQuantity,
    pub transient_keypair: KeyPair,
}

impl BitcoinRefund {
    pub fn to_transaction(
        &self,
        to_address: bitcoin_support::Address,
    ) -> bitcoin_witness::PrimedTransaction {
        bitcoin_witness::PrimedTransaction {
            inputs: vec![bitcoin_witness::PrimedInput::new(
                self.htlc_id.transaction_id.clone().into(),
                self.htlc_id.vout,
                self.value,
                self.htlc.unlock_after_timeout(self.transient_keypair),
            )],
            locktime: 0,
            output_address: to_address,
        }
    }
}

struct EthereumDeploy {
    data: ethereum_support::Bytes,
    value: ethereum_support::EtherQuantity,
    gas_limit: u32,
}

struct EtherRedeem {
    pub contract_address: ethereum_support::Address,
    pub execution_gas: u32,
    pub data: Secret,
}

pub mod btc_eth;
pub mod eth_btc;
