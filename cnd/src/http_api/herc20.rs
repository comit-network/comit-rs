use crate::{
    actions::ethereum::{CallContract, DeployContract},
    asset,
    ethereum::ChainId,
    identity, Secret, SecretHash, Timestamp,
};

pub use crate::herc20::*;

/// Data for the herc20 protocol, wrapped where needed to control
/// serialization/deserialization.
#[derive(serde::Deserialize, Clone, Debug)]
pub struct Herc20 {
    pub amount: asset::Erc20Quantity,
    pub identity: identity::Ethereum,
    pub chain_id: ChainId,
    pub token_contract: identity::Ethereum,
    pub absolute_expiry: u32,
}

impl From<Herc20> for CreatedSwap {
    fn from(p: Herc20) -> Self {
        CreatedSwap {
            asset: asset::Erc20::new(p.token_contract, p.amount),
            identity: p.identity,
            chain_id: p.chain_id,
            absolute_expiry: p.absolute_expiry,
        }
    }
}

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
        let params = self.build_params(secret_hash);
        params.build_deploy_action()
    }

    pub fn build_fund_action(&self, secret_hash: SecretHash) -> anyhow::Result<CallContract> {
        let htlc_location = match self.state {
            State::Deployed { htlc_location, .. } => htlc_location,
            _ => anyhow::bail!("incorrect state"),
        };

        let params = self.build_params(secret_hash);
        Ok(params.build_fund_action(htlc_location))
    }

    pub fn build_refund_action(&self, secret_hash: SecretHash) -> anyhow::Result<CallContract> {
        let htlc_location = match self.state {
            State::Funded { htlc_location, .. } => htlc_location,
            _ => anyhow::bail!("incorrect state"),
        };

        let params = self.build_params(secret_hash);
        Ok(params.build_refund_action(htlc_location))
    }

    pub fn build_redeem_action(&self, secret: Secret) -> anyhow::Result<CallContract> {
        let htlc_location = match self.state {
            State::Funded { htlc_location, .. } => htlc_location,
            _ => anyhow::bail!("incorrect state"),
        };

        let secret_hash = SecretHash::new(secret);
        let params = self.build_params(secret_hash);
        Ok(params.build_redeem_action(htlc_location, secret))
    }

    fn build_params(&self, secret_hash: SecretHash) -> Params {
        Params {
            asset: self.asset.clone(),
            redeem_identity: self.redeem_identity,
            refund_identity: self.refund_identity,
            expiry: self.expiry,
            secret_hash,
            chain_id: self.chain_id,
        }
    }
}
