use crate::swap::hbit;
use std::sync::Arc;

pub use crate::bitcoin::Amount;
pub use ::bitcoin::{Address, Block, BlockHash, OutPoint};
use comit::asset;

#[derive(Debug, Clone)]
pub struct Wallet {
    pub inner: crate::bitcoin_wallet::Wallet,
    pub connector: Arc<comit::btsieve::bitcoin::BitcoindConnector>,
}

#[async_trait::async_trait]
impl hbit::ExecuteFund for Wallet {
    async fn execute_fund(
        &self,
        params: &comit::hbit::Params,
    ) -> anyhow::Result<hbit::CorrectlyFunded> {
        let action = params.build_fund_action();

        let txid = self
            .inner
            .send_to_address(action.to, action.amount.into(), action.network)
            .await?;
        let transaction = self.inner.get_raw_transaction(txid).await?;

        // TODO: This code is copied straight from COMIT lib. We
        // should find a way of not having to duplicate this logic
        let location = transaction
            .output
            .iter()
            .enumerate()
            .map(|(index, txout)| {
                // Casting a usize to u32 can lead to truncation on 64bit platforms
                // However, bitcoin limits the number of inputs to u32 anyway, so this
                // is not a problem for us.
                #[allow(clippy::cast_possible_truncation)]
                (index as u32, txout)
            })
            .find(|(_, txout)| txout.script_pubkey == params.compute_address().script_pubkey())
            .map(|(vout, _txout)| bitcoin::OutPoint { txid, vout });

        let location = location.ok_or_else(|| {
            anyhow::anyhow!("Fund transaction does not contain expected outpoint")
        })?;
        let asset = asset::Bitcoin::from_sat(transaction.output[location.vout as usize].value);

        Ok(hbit::CorrectlyFunded { asset, location })
    }
}

impl Wallet {
    pub async fn redeem(
        &self,
        action: hbit::BroadcastSignedTransaction,
    ) -> anyhow::Result<bitcoin::Transaction> {
        self.spend(action).await
    }

    pub async fn refund(
        &self,
        action: hbit::BroadcastSignedTransaction,
    ) -> anyhow::Result<bitcoin::Transaction> {
        self.spend(action).await
    }

    async fn spend(
        &self,
        action: hbit::BroadcastSignedTransaction,
    ) -> anyhow::Result<bitcoin::Transaction> {
        let _txid = self
            .inner
            .send_raw_transaction(action.transaction.clone(), action.network)
            .await?;

        Ok(action.transaction)
    }
}
