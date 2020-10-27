//! Implement traits to Load/Save types defined in the http_api module.
use crate::{
    http_api::{Protocol, SwapEvent, SwapResource},
    storage::{Hbit, Herc20, Load, LoadTables, SwapContext, Tables},
    LocalSwapId, Storage,
};
use anyhow::Result;
use async_trait::async_trait;
use comit::LockProtocol;

#[async_trait]
impl Load<SwapResource> for Storage {
    async fn load(&self, swap_id: LocalSwapId) -> Result<SwapResource> {
        let context: SwapContext = self.load(swap_id).await?;

        let (alpha, beta) = match context {
            SwapContext {
                alpha: LockProtocol::Hbit,
                beta: LockProtocol::Herc20,
                ..
            } => {
                let tab: Tables<Hbit, Herc20> = self.db.load_tables(swap_id).await?;

                (
                    Protocol::hbit(tab.alpha.into()),
                    Protocol::herc20_dai(tab.beta.amount),
                )
            }
            SwapContext {
                alpha: LockProtocol::Herc20,
                beta: LockProtocol::Hbit,
                ..
            } => {
                let tab: Tables<Herc20, Hbit> = self.db.load_tables(swap_id).await?;

                (
                    Protocol::herc20_dai(tab.alpha.amount),
                    Protocol::hbit(tab.beta.into()),
                )
            }
            _ => anyhow::bail!("unsupported combination of locking protocols"),
        };

        let mut swap_events = Vec::with_capacity(5);

        if let Some(events) = self.hbit_events.lock().await.get(&swap_id) {
            if let Some(tx) = events.fund.map(|e| e.location.txid) {
                swap_events.push(SwapEvent::HbitFunded { tx })
            }

            if let Some(tx) = events.redeem.map(|e| e.transaction) {
                swap_events.push(SwapEvent::HbitRedeemed { tx })
            }
        }
        if let Some(events) = self.herc20_events.lock().await.get(&swap_id) {
            if let Some(tx) = events.deploy.as_ref().map(|e| e.transaction) {
                swap_events.push(SwapEvent::Herc20Deployed { tx })
            }

            if let Some(tx) = events.fund.as_ref().map(|e| e.transaction) {
                swap_events.push(SwapEvent::Herc20Funded { tx })
            }

            if let Some(tx) = events.redeem.as_ref().map(|e| e.transaction) {
                swap_events.push(SwapEvent::Herc20Redeemed { tx })
            }
        }

        Ok(SwapResource {
            role: context.role,
            events: swap_events,
            alpha,
            beta,
        })
    }
}
