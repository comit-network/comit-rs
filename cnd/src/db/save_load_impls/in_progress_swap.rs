use crate::{
    db::{InProgressSwap, Load, Sqlite},
    swap_protocols::{halight, herc20, Ledger, LocalSwapId},
};
use async_trait::async_trait;

// This is tested in db/integration_tests/api.rs
#[async_trait]
impl Load<InProgressSwap<herc20::InProgressSwap, halight::InProgressSwap>> for Sqlite {
    async fn load(
        &self,
        swap_id: LocalSwapId,
    ) -> anyhow::Result<Option<InProgressSwap<herc20::InProgressSwap, halight::InProgressSwap>>>
    {
        let swap = self.load_swap(swap_id).await?;
        let herc20 = self.load_herc20(swap_id).await?;
        let halight = self.load_halight(swap_id).await?;

        let role = swap.role.0;

        let alpha_refund_identity = match herc20.refund_identity {
            Some(id) => id.0,
            None => return Ok(None),
        };
        let alpha_redeem_identity = match herc20.redeem_identity {
            Some(id) => id.0,
            None => return Ok(None),
        };
        let beta_refund_identity = match halight.refund_identity {
            Some(id) => id.0,
            None => return Ok(None),
        };
        let beta_redeem_identity = match halight.redeem_identity {
            Some(id) => id.0,
            None => return Ok(None),
        };

        let live = InProgressSwap {
            swap_id,
            role,
            alpha: herc20::InProgressSwap {
                ledger: Ledger::Alpha,
                refund_identity: alpha_refund_identity.into(),
                redeem_identity: alpha_redeem_identity.into(),
                expiry: herc20.expiry.into(),
            },
            beta: halight::InProgressSwap {
                ledger: Ledger::Beta,
                asset: halight.amount.0.into(),
                refund_identity: beta_refund_identity,
                redeem_identity: beta_redeem_identity,
                expiry: halight.cltv_expiry.into(),
            },
        };

        Ok(Some(live))
    }
}
