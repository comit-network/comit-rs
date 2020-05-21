use crate::{
    db,
    db::{
        tables::{Insert, InsertableSwap, IntoInsertable},
        wrapper_types::{custom_sql_types::Text, Erc20Amount, EthereumAddress, Satoshis},
        CreatedSwap, ForSwap, Load, Save, Sqlite,
    },
    http_api,
    swap_protocols::{halight, herc20, LocalSwapId, Role, Side},
};
use anyhow::Context;
use comit::{asset, asset::Erc20, network, Protocol};
use diesel::{sql_types, ExpressionMethods, JoinOnDsl, QueryDsl, RunQueryDsl};

mod rfc003;

#[async_trait::async_trait]
impl<TCreatedA, TCreatedB, TInsertableA, TInsertableB> Save<CreatedSwap<TCreatedA, TCreatedB>>
    for Sqlite
where
    TCreatedA: IntoInsertable<Insertable = TInsertableA> + Clone + Send + 'static,
    TCreatedB: IntoInsertable<Insertable = TInsertableB> + Send + 'static,
    TInsertableA: 'static,
    TInsertableB: 'static,
    Sqlite: Insert<TInsertableA> + Insert<TInsertableB>,
{
    async fn save(
        &self,
        CreatedSwap {
            swap_id,
            role,
            peer,
            alpha,
            beta,
            ..
        }: CreatedSwap<TCreatedA, TCreatedB>,
    ) -> anyhow::Result<()> {
        self.do_in_transaction::<_, _, anyhow::Error>(move |conn| {
            let swap_id = self.save_swap(conn, &InsertableSwap::new(swap_id, peer, role))?;

            let insertable_alpha = alpha.into_insertable(swap_id, role, Side::Alpha);
            let insertable_beta = beta.into_insertable(swap_id, role, Side::Beta);

            self.insert(conn, &insertable_alpha)?;
            self.insert(conn, &insertable_beta)?;

            Ok(())
        })
        .await?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl Load<http_api::Swap<herc20::Asset, halight::Asset>> for Sqlite {
    async fn load(
        &self,
        swap_id: LocalSwapId,
    ) -> anyhow::Result<http_api::Swap<herc20::Asset, halight::Asset>> {
        use crate::db::schema::{halights, herc20s, swaps};

        let (role, erc20_amount, token_contract, satoshis) = self
            .do_in_transaction(move |conn| {
                let key = Text(swap_id);

                swaps::table
                    .inner_join(halights::table.on(swaps::id.eq(halights::swap_id)))
                    .inner_join(herc20s::table.on(swaps::id.eq(herc20s::swap_id)))
                    .filter(swaps::local_swap_id.eq(key))
                    .select((
                        swaps::role,
                        herc20s::amount,
                        herc20s::token_contract,
                        halights::amount,
                    ))
                    .first::<(
                        Text<Role>,
                        Text<Erc20Amount>,
                        Text<EthereumAddress>,
                        Text<Satoshis>,
                    )>(conn)
            })
            .await
            .map_err(|_| db::Error::SwapNotFound)?;

        let swap = http_api::Swap {
            role: role.0,
            alpha: herc20::Asset(asset::Erc20 {
                token_contract: token_contract.0.into(),
                quantity: erc20_amount.0.into(),
            }),
            beta: halight::Asset(satoshis.0.into()),
        };

        Ok(swap)
    }
}

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
            side: herc20.side.0,
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
            side: halight.side.0,
            asset: halight.amount.0.into(),
            refund_identity: refund_identity.0,
            redeem_identity: redeem_identity.0,
            expiry: halight.cltv_expiry.into(),
        })
    }
}

#[async_trait::async_trait]
impl Load<http_api::Swap<comit::Protocol, comit::Protocol>> for Sqlite {
    async fn load(
        &self,
        swap_id: LocalSwapId,
    ) -> anyhow::Result<http_api::Swap<comit::Protocol, comit::Protocol>> {
        #[derive(QueryableByName)]
        struct Result {
            #[sql_type = "sql_types::Text"]
            role: Text<Role>,
            #[sql_type = "sql_types::Text"]
            alpha_protocol: Text<Protocol>,
            #[sql_type = "sql_types::Text"]
            beta_protocol: Text<Protocol>,
        }

        let Result { role, alpha_protocol, beta_protocol } = self.do_in_transaction(|connection| {
            // Here is how this works:
            // - COALESCE selects the first non-null value from a list of values
            // - We use 3 sub-selects to select a static value (i.e. 'halight', etc) if that particular child table has a row with a foreign key to the parent table
            // - We do this two times, once where we limit the results to rows that have `ledger` set to `Alpha` and once where `ledger` is set to `Beta`
            // 
            // The result is a view with 3 columns: `role`, `alpha_protocol` and `beta_protocol` where the `*_protocol` columns have one of the values `halight`, `herc20` or `hbit`
            diesel::sql_query(
                r#"
                SELECT
                    role,
                    COALESCE(
                       (SELECT 'halight' from halights where halights.swap_id = swaps.id and halights.side = 'Alpha'),
                       (SELECT 'herc20' from herc20s where herc20s.swap_id = swaps.id and herc20s.side = 'Alpha'),
                       (SELECT 'hbit' from hbits where hbits.swap_id = swaps.id and hbits.side = 'Alpha')
                    ) as alpha_protocol,
                    COALESCE(
                       (SELECT 'halight' from halights where halights.swap_id = swaps.id and halights.side = 'Beta'),
                       (SELECT 'herc20' from herc20s where herc20s.swap_id = swaps.id and herc20s.side = 'Beta'),
                       (SELECT 'hbit' from hbits where hbits.swap_id = swaps.id and hbits.side = 'Beta')
                    ) as beta_protocol
                from swaps
                    where local_swap_id = ?
            "#,
            )
                .bind::<sql_types::Text, _>(Text(swap_id))
                .get_result(connection)
        }).await.context(db::Error::SwapNotFound)?;

        Ok(http_api::Swap {
            role: role.0,
            alpha: alpha_protocol.0,
            beta: beta_protocol.0,
        })
    }
}
