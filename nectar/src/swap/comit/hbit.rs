use crate::swap::comit::SwapFailedShouldRefund;
use anyhow::Result;
use bitcoin::{secp256k1::SecretKey, Block, BlockHash};
use chrono::{DateTime, Utc};
use comit::asset;

pub use comit::{
    actions::bitcoin::{BroadcastSignedTransaction, SendToAddress},
    btsieve::{BlockByHash, LatestBlock},
    hbit::*,
    htlc_location, transaction, Secret, SecretHash, Timestamp,
};

pub type SharedParams = comit::hbit::Params;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Params {
    pub shared: SharedParams,
    pub transient_sk: SecretKey,
}

impl Params {
    pub fn new(shared: SharedParams, transient_sk: SecretKey) -> Self {
        Self {
            shared,
            transient_sk,
        }
    }
}

#[async_trait::async_trait]
pub trait ExecuteFund {
    async fn execute_fund(&self, params: &Params) -> Result<Funded>;
}

#[async_trait::async_trait]
pub trait ExecuteRedeem {
    async fn execute_redeem(
        &self,
        params: Params,
        fund_event: Funded,
        secret: Secret,
    ) -> Result<Redeemed>;
}

#[async_trait::async_trait]
pub trait ExecuteRefund {
    async fn execute_refund(&self, params: Params, fund_event: Funded) -> Result<Refunded>;
}

#[derive(Debug, Clone, Copy)]
pub struct Funded {
    pub asset: asset::Bitcoin,
    pub location: htlc_location::Bitcoin,
}

pub async fn watch_for_funded<C>(
    connector: &C,
    params: &SharedParams,
    utc_start_of_swap: DateTime<Utc>,
) -> Result<Funded>
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = BlockHash>,
{
    match comit::hbit::watch_for_funded(connector, &params, utc_start_of_swap).await? {
        comit::hbit::Funded::Correctly {
            asset, location, ..
        } => Ok(Funded { asset, location }),
        comit::hbit::Funded::Incorrectly { .. } => anyhow::bail!("Bitcoin HTLC incorrectly funded"),
    }
}

/// Executes refund if deemed necessary based on the result of the swap.
pub async fn refund_if_necessary<A>(actor: A, hbit: Params, swap_result: Result<()>) -> Result<()>
where
    A: ExecuteRefund,
{
    if let Err(e) = swap_result {
        if let Some(swap_failed) = e.downcast_ref::<SwapFailedShouldRefund<Funded>>() {
            actor.execute_refund(hbit, swap_failed.0).await?;
        }

        return Err(e);
    }

    Ok(())
}

#[cfg(test)]
mod arbitrary {
    use crate::swap::hbit::{Params, SharedParams};
    use ::bitcoin::secp256k1::SecretKey;
    use comit::{asset, identity, ledger};
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for Params {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Params {
                shared: SharedParams {
                    network: bitcoin_network(g),
                    asset: bitcoin_asset(g),
                    redeem_identity: bitcoin_identity(g),
                    refund_identity: bitcoin_identity(g),
                    expiry: crate::arbitrary::timestamp(g),
                    secret_hash: crate::arbitrary::secret_hash(g),
                },
                transient_sk: secret_key(g),
            }
        }
    }

    fn secret_key<G: Gen>(g: &mut G) -> SecretKey {
        let mut bytes = [0u8; 32];
        for byte in &mut bytes {
            *byte = u8::arbitrary(g);
        }
        SecretKey::from_slice(&bytes).unwrap()
    }

    fn bitcoin_network<G: Gen>(g: &mut G) -> ledger::Bitcoin {
        match u8::arbitrary(g) % 3 {
            0 => ledger::Bitcoin::Mainnet,
            1 => ledger::Bitcoin::Testnet,
            2 => ledger::Bitcoin::Regtest,
            _ => unreachable!(),
        }
    }

    fn bitcoin_asset<G: Gen>(g: &mut G) -> asset::Bitcoin {
        asset::Bitcoin::from_sat(u64::arbitrary(g))
    }

    fn bitcoin_identity<G: Gen>(g: &mut G) -> identity::Bitcoin {
        identity::Bitcoin::from_secret_key(&crate::SECP, &secret_key(g))
    }
}
