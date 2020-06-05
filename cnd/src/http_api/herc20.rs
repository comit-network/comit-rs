use crate::{
    actions::ethereum::{CallContract, DeployContract},
    asset,
    ethereum::{Bytes, ChainId},
    identity, Secret, SecretHash, Timestamp,
};
use blockchain_contracts::ethereum::rfc003::{Erc20Htlc, EtherHtlc};

pub use crate::herc20::*;

#[derive(Clone, Debug)]
pub struct Finalized {
    pub asset: asset::Erc20,
    pub chain_id: ChainId,
    pub refund_identity: identity::Ethereum,
    pub redeem_identity: identity::Ethereum,
    pub expiry: Timestamp,
    pub state: State,
}

impl Finalized {
    pub fn build_deploy_action(&self, secret_hash: SecretHash) -> DeployContract {
        let htlc = build_erc20_htlc(
            self.asset.clone(),
            self.redeem_identity,
            self.refund_identity,
            self.expiry,
            secret_hash,
        );
        let gas_limit = Erc20Htlc::deploy_tx_gas_limit();

        DeployContract {
            data: htlc.into(),
            amount: asset::Ether::zero(),
            gas_limit,
            chain_id: self.chain_id,
        }
    }

    pub fn build_fund_action(&self) -> anyhow::Result<CallContract> {
        let htlc_location = match self.state {
            State::Deployed { htlc_location, .. } => htlc_location,
            _ => anyhow::bail!("incorrect state"),
        };

        let to = self.asset.token_contract;
        let htlc_address = blockchain_contracts::ethereum::Address(htlc_location.into());
        let data =
            Erc20Htlc::transfer_erc20_tx_payload(self.asset.clone().quantity.into(), htlc_address);
        let data = Some(Bytes(data));

        let gas_limit = Erc20Htlc::fund_tx_gas_limit();
        let min_block_timestamp = None;

        Ok(CallContract {
            to,
            data,
            gas_limit,
            chain_id: self.chain_id,
            min_block_timestamp,
        })
    }

    pub fn build_refund_action(&self) -> anyhow::Result<CallContract> {
        let to = match self.state {
            State::Funded { htlc_location, .. } => htlc_location,
            _ => anyhow::bail!("incorrect state"),
        };
        let data = None;
        let gas_limit = EtherHtlc::refund_tx_gas_limit();
        let min_block_timestamp = Some(self.expiry);

        Ok(CallContract {
            to,
            data,
            gas_limit,
            chain_id: self.chain_id,
            min_block_timestamp,
        })
    }

    pub fn build_redeem_action(&self, secret: Secret) -> anyhow::Result<CallContract> {
        let to = match self.state {
            State::Funded { htlc_location, .. } => htlc_location,
            _ => anyhow::bail!("incorrect state"),
        };
        let data = Some(Bytes::from(secret.into_raw_secret().to_vec()));
        let gas_limit = EtherHtlc::redeem_tx_gas_limit();
        let min_block_timestamp = None;

        Ok(CallContract {
            to,
            data,
            gas_limit,
            chain_id: self.chain_id,
            min_block_timestamp,
        })
    }
}
