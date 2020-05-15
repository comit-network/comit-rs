use crate::{
    db,
    db::{wrapper_types::custom_sql_types::Text, ForSwap, Load, Save, Sqlite},
    swap_protocols::{halight, herc20, LocalSwapId, Role},
};
use comit::{asset::Erc20, network};
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};

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

#[async_trait::async_trait]
impl Load<Role> for Sqlite {
    async fn load(&self, swap_id: LocalSwapId) -> anyhow::Result<Role> {
        use crate::db::schema::swaps;

        let role = self
            .do_in_transaction(move |conn| {
                let key = Text(swap_id);

                swaps::table
                    .filter(swaps::local_swap_id.eq(key))
                    .select(swaps::role)
                    .first::<Text<Role>>(conn)
            })
            .await
            .map_err(|_| db::Error::SwapNotFound)?;

        Ok(role.0)
    }
}

#[async_trait::async_trait]
impl Load<herc20::InProgressSwap> for Sqlite {
    async fn load(&self, swap_id: LocalSwapId) -> anyhow::Result<herc20::InProgressSwap> {
        let herc20 = self.load_herc20(swap_id).await?;

        let refund_identity = herc20.refund_identity.ok_or(db::Error::IdentityNotSet)?;
        let redeem_identity = herc20.redeem_identity.ok_or(db::Error::IdentityNotSet)?;

        Ok(herc20::InProgressSwap {
            asset: Erc20::new(herc20.token_contract.0.into(), herc20.amount.0.into()),
            ledger: herc20.ledger.0,
            refund_identity: refund_identity.0.into(),
            redeem_identity: redeem_identity.0.into(),
            expiry: herc20.expiry.into(),
        })
    }
}

#[async_trait::async_trait]
impl Load<halight::InProgressSwap> for Sqlite {
    async fn load(&self, swap_id: LocalSwapId) -> anyhow::Result<halight::InProgressSwap> {
        let halight = self.load_halight(swap_id).await?;

        let refund_identity = halight.refund_identity.ok_or(db::Error::IdentityNotSet)?;
        let redeem_identity = halight.redeem_identity.ok_or(db::Error::IdentityNotSet)?;

        Ok(halight::InProgressSwap {
            ledger: halight.ledger.0,
            asset: halight.amount.0.into(),
            refund_identity: refund_identity.0,
            redeem_identity: redeem_identity.0,
            expiry: halight.cltv_expiry.into(),
        })
    }
}
