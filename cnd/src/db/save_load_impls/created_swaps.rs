use crate::{
    db::{
        tables::{Insert, InsertableSwap, IntoInsertable},
        CreatedSwap, Error, Load, Save, Sqlite,
    },
    swap_protocols::{halight, herc20, Ledger, LocalSwapId, Role},
};
use async_trait::async_trait;

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
impl Load<CreatedSwap<herc20::CreatedSwap, halight::CreatedSwap>> for Sqlite {
    async fn load(
        &self,
        swap_id: LocalSwapId,
    ) -> anyhow::Result<Option<CreatedSwap<herc20::CreatedSwap, halight::CreatedSwap>>> {
        let swap = self.load_swap(swap_id).await?;
        let herc20 = self.load_herc20(swap_id).await?;
        let halight = self.load_halight(swap_id).await?;

        let role = swap.role.0;
        let peer = swap.counterparty_peer_id.0;

        let address_hint = self.load_address_hint(&peer).await.ok();

        let alpha_identity = match role {
            Role::Alice => herc20
                .refund_identity
                .ok_or_else(|| Error::IdentityNotSet)?,
            Role::Bob => herc20
                .redeem_identity
                .ok_or_else(|| Error::IdentityNotSet)?,
        };
        let beta_identity = match role {
            Role::Alice => halight
                .redeem_identity
                .ok_or_else(|| Error::IdentityNotSet)?,
            Role::Bob => halight
                .refund_identity
                .ok_or_else(|| Error::IdentityNotSet)?,
        };

        let alpha = herc20::CreatedSwap {
            amount: herc20.amount.0.into(),
            identity: alpha_identity.0.into(),
            chain_id: herc20.chain_id.into(),
            token_contract: herc20.token_contract.0.into(),
            absolute_expiry: herc20.expiry.into(),
        };

        let beta = halight::CreatedSwap {
            amount: halight.amount.0.into(),
            identity: beta_identity.0,
            network: halight.network.0.into(),
            cltv_expiry: halight.cltv_expiry.into(),
        };

        let created = CreatedSwap {
            swap_id,
            alpha,
            beta,
            peer,
            address_hint,
            role,
        };

        Ok(Some(created))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        asset,
        db::{
            wrapper_types::{Erc20Amount, EthereumAddress},
            Sqlite,
        },
        identity,
        swap_protocols::ledger,
        timestamp::Timestamp,
    };
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
    async fn roundtrip_created_swap() {
        let path = temp_db();
        let db = Sqlite::new(&path).expect("a new db");

        let local_swap_id =
            LocalSwapId::from_str("111152ca-ecf2-4cc6-b35c-b4351ac28a34").expect("valid swap id");
        let role = Role::Alice;
        let peer = PeerId::from_str("QmfUfpC2frwFvcDzpspnfZitHt5wct6n4kpG5jzgRdsxkY")
            .expect("valid peer id");

        let alpha_amount = Erc20Amount::from_str("12345").expect("valid ERC20 amount");
        let token_contract = EthereumAddress::from_str("1111e8be41b21f651a71aaB1A85c6813b8bBcCf8")
            .expect("valid etherum identity");
        let alpha_identity = EthereumAddress::from_str("2222e8be41b21f651a71aaB1A85c6813b8bBcCf8")
            .expect("valid redeem identity");
        let alpha_expiry = Timestamp::from(123u32);

        let beta_amount = asset::Bitcoin::from_sat(999);
        let beta_identity = identity::Lightning::random();
        let beta_expiry = Timestamp::from(456u32);

        let created: CreatedSwap<herc20::CreatedSwap, halight::CreatedSwap> = CreatedSwap {
            swap_id: local_swap_id,
            alpha: herc20::CreatedSwap {
                amount: alpha_amount.into(),
                identity: alpha_identity.into(),
                chain_id: 1337,
                token_contract: token_contract.into(),
                absolute_expiry: alpha_expiry.into(),
            },
            beta: halight::CreatedSwap {
                amount: beta_amount,
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

        let loaded = Load::<CreatedSwap<herc20::CreatedSwap, halight::CreatedSwap>>::load(
            &db,
            local_swap_id,
        )
        .await
        .expect("to be able to load previously save created swap")
        .expect("some created swap");

        assert_eq!(loaded, created);
    }
}
