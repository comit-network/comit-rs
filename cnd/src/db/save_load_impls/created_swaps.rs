use crate::{
    asset,
    db::{
        self,
        tables::{Insert, InsertableSwap, IntoInsertable},
        wrapper_types::{custom_sql_types::Text, Erc20Amount, EthereumAddress, Satoshis},
        CreatedSwap, Load, Save, Sqlite,
    },
    http_api,
    swap_protocols::{halight, herc20, Ledger, LocalSwapId, Role},
};
use async_trait::async_trait;
use diesel::prelude::*;

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

            let insertable_alpha = alpha.into_insertable(swap_id, role, Ledger::Alpha);
            let insertable_beta = beta.into_insertable(swap_id, role, Ledger::Beta);

            self.insert(conn, &insertable_alpha)?;
            self.insert(conn, &insertable_beta)?;

            Ok(())
        })
        .await?;

        Ok(())
    }
}

#[async_trait]
impl Load<http_api::DisplaySwap<herc20::Asset, halight::Asset>> for Sqlite {
    async fn load(
        &self,
        swap_id: LocalSwapId,
    ) -> anyhow::Result<http_api::DisplaySwap<herc20::Asset, halight::Asset>> {
        use crate::db::schema::{halights, herc20s, swaps};

        let (role, satoshis, erc20_amount, token_contract) = self
            .do_in_transaction(move |conn| {
                let key = Text(swap_id);

                swaps::table
                    .inner_join(halights::table.on(swaps::id.eq(halights::swap_id)))
                    .inner_join(herc20s::table.on(swaps::id.eq(herc20s::swap_id)))
                    .filter(swaps::local_swap_id.eq(key))
                    .select((
                        swaps::role,
                        halights::amount,
                        herc20s::amount,
                        herc20s::token_contract,
                    ))
                    .first::<(
                        Text<Role>,
                        Text<Satoshis>,
                        Text<Erc20Amount>,
                        Text<EthereumAddress>,
                    )>(conn)
            })
            .await
            .map_err(|_| db::Error::SwapNotFound)?;

        let swap = http_api::DisplaySwap {
            role: role.0,
            alpha_asset: herc20::Asset(asset::Erc20 {
                token_contract: token_contract.0.into(),
                quantity: erc20_amount.0.into(),
            }),
            beta_asset: halight::Asset(satoshis.0.into()),
        };

        Ok(swap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        asset,
        db::Sqlite,
        identity,
        swap_protocols::ledger,
        timestamp::{RelativeTime, Timestamp},
    };
    use comit::asset::Erc20Quantity;
    use libp2p::PeerId;
    use std::{path::PathBuf, str::FromStr};

    fn temp_db() -> PathBuf {
        let temp_file = tempfile::Builder::new()
            .suffix(".sqlite")
            .tempfile()
            .unwrap();

        temp_file.into_temp_path().to_path_buf()
    }

    #[tokio::test]
    async fn saved_created_swap() {
        let path = temp_db();
        let db = Sqlite::new(&path).expect("a new db");

        let local_swap_id =
            LocalSwapId::from_str("111152ca-ecf2-4cc6-b35c-b4351ac28a34").expect("valid swap id");
        let role = Role::Alice;
        let peer = PeerId::from_str("QmfUfpC2frwFvcDzpspnfZitHt5wct6n4kpG5jzgRdsxkY")
            .expect("valid peer id");

        let alpha_asset = asset::Erc20::new(
            identity::Ethereum::from_str("1111e8be41b21f651a71aaB1A85c6813b8bBcCf8").unwrap(),
            Erc20Quantity::from_wei_dec_str("12345").unwrap(),
        );
        let alpha_identity =
            identity::Ethereum::from_str("2222e8be41b21f651a71aaB1A85c6813b8bBcCf8")
                .expect("valid redeem identity");
        let alpha_expiry = Timestamp::from(123u32);

        let beta_asset = asset::Bitcoin::from_sat(999);
        let beta_identity = identity::Lightning::random();
        let beta_expiry = RelativeTime::from(456u32);

        let created: CreatedSwap<herc20::CreatedSwap, halight::CreatedSwap> = CreatedSwap {
            swap_id: local_swap_id,
            alpha: herc20::CreatedSwap {
                asset: alpha_asset,
                identity: alpha_identity,
                chain_id: 1337,
                absolute_expiry: alpha_expiry.into(),
            },
            beta: halight::CreatedSwap {
                asset: beta_asset,
                identity: beta_identity,
                network: ledger::Lightning::Regtest,
                cltv_expiry: beta_expiry.into(),
            },
            peer,
            address_hint: None,
            role,
        };

        db.save(created.clone())
            .await
            .expect("to be able to save created swap");
    }
}
