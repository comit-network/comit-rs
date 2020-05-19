use crate::db::{ForSwap, Save, Sqlite};
use comit::network;

mod created_swaps;
mod rfc003;

#[async_trait::async_trait]
impl Save<ForSwap<network::WhatAliceLearnedFromBob>> for Sqlite {
    async fn save(&self, swap: ForSwap<network::WhatAliceLearnedFromBob>) -> anyhow::Result<()> {
        let local_swap_id = swap.local_swap_id;
        let refund_lightning_identity = swap.data.refund_lightning_identity;
        let redeem_ethereum_identity = swap.data.redeem_ethereum_identity;

        self.do_in_transaction(|conn| {
            self.update_halight_refund_identity(conn, local_swap_id, refund_lightning_identity)?;
            self.update_herc20_redeem_identity(conn, local_swap_id, redeem_ethereum_identity)?;

            Ok(())
        })
        .await
    }
}

#[async_trait::async_trait]
impl Save<ForSwap<network::WhatBobLearnedFromAlice>> for Sqlite {
    async fn save(&self, swap: ForSwap<network::WhatBobLearnedFromAlice>) -> anyhow::Result<()> {
        let local_swap_id = swap.local_swap_id;
        let redeem_lightning_identity = swap.data.redeem_lightning_identity;
        let refund_ethereum_identity = swap.data.refund_ethereum_identity;
        let secret_hash = swap.data.secret_hash;

        self.do_in_transaction(|conn| {
            self.update_halight_redeem_identity(conn, local_swap_id, redeem_lightning_identity)?;
            self.update_herc20_refund_identity(conn, local_swap_id, refund_ethereum_identity)?;
            self.insert_secret_hash(conn, local_swap_id, secret_hash)?;

            Ok(())
        })
        .await
    }
}
