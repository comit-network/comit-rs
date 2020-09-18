use crate::{
    database::{serialize, Database, Load, Save},
    swap::hbit,
    SwapId,
};
use ::bitcoin::secp256k1;
use anyhow::{anyhow, Context};
use comit::{identity, Secret, SecretHash, Timestamp};
use serde::{Deserialize, Serialize};

// TODO: control the serialisation
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct HbitFunded {
    pub asset: Amount,
    pub location: ::bitcoin::OutPoint,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Amount(u64);

impl From<Amount> for comit::asset::Bitcoin {
    fn from(amount: Amount) -> Self {
        comit::asset::Bitcoin::from_sat(amount.0)
    }
}

impl From<comit::asset::Bitcoin> for Amount {
    fn from(amount: comit::asset::Bitcoin) -> Self {
        Amount(amount.as_sat())
    }
}

impl From<HbitFunded> for hbit::Funded {
    fn from(funded: HbitFunded) -> Self {
        hbit::Funded {
            asset: funded.asset.into(),
            location: funded.location,
        }
    }
}

impl From<hbit::Funded> for HbitFunded {
    fn from(funded: hbit::Funded) -> Self {
        HbitFunded {
            asset: funded.asset.into(),
            location: funded.location,
        }
    }
}

#[async_trait::async_trait]
impl Save<hbit::Funded> for Database {
    async fn save(&self, event: hbit::Funded, swap_id: SwapId) -> anyhow::Result<()> {
        let stored_swap = self.get_swap(&swap_id)?;

        match stored_swap.hbit_funded {
            Some(_) => Err(anyhow!("Hbit Funded event is already stored")),
            None => {
                let key = serialize(&swap_id)?;

                let mut swap = stored_swap.clone();
                swap.hbit_funded = Some(event.into());

                let old_value =
                    serialize(&stored_swap).context("Could not serialize old swap value")?;
                let new_value = serialize(&swap).context("Could not serialize new swap value")?;

                self.db
                    .compare_and_swap(key, Some(old_value), Some(new_value))
                    .context("Could not write in the DB")?
                    .context("Stored swap somehow changed, aborting saving")?;

                self.db
                    .flush_async()
                    .await
                    .map(|_| ())
                    .context("Could not flush db")
            }
        }
    }
}

impl Load<hbit::Funded> for Database {
    fn load(&self, swap_id: SwapId) -> anyhow::Result<Option<hbit::Funded>> {
        let swap = self.get_swap(&swap_id)?;

        Ok(swap.hbit_funded.map(Into::into))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HbitRedeemed {
    pub transaction: comit::transaction::Bitcoin,
    pub secret: Secret,
}

impl From<HbitRedeemed> for hbit::Redeemed {
    fn from(event: HbitRedeemed) -> Self {
        hbit::Redeemed {
            transaction: event.transaction,
            secret: event.secret,
        }
    }
}

impl From<hbit::Redeemed> for HbitRedeemed {
    fn from(event: hbit::Redeemed) -> Self {
        HbitRedeemed {
            transaction: event.transaction,
            secret: event.secret,
        }
    }
}

#[async_trait::async_trait]
impl Save<hbit::Redeemed> for Database {
    async fn save(&self, event: hbit::Redeemed, swap_id: SwapId) -> anyhow::Result<()> {
        let stored_swap = self.get_swap(&swap_id)?;

        match stored_swap.hbit_redeemed {
            Some(_) => Err(anyhow!("Hbit Redeemed event is already stored")),
            None => {
                let key = serialize(&swap_id)?;

                let mut swap = stored_swap.clone();
                swap.hbit_redeemed = Some(event.into());

                let old_value =
                    serialize(&stored_swap).context("Could not serialize old swap value")?;
                let new_value = serialize(&swap).context("Could not serialize new swap value")?;

                self.db
                    .compare_and_swap(key, Some(old_value), Some(new_value))
                    .context("Could not write in the DB")?
                    .context("Stored swap somehow changed, aborting saving")?;

                self.db
                    .flush_async()
                    .await
                    .map(|_| ())
                    .context("Could not flush db")
            }
        }
    }
}

impl Load<hbit::Redeemed> for Database {
    fn load(&self, swap_id: SwapId) -> anyhow::Result<Option<hbit::Redeemed>> {
        let swap = self.get_swap(&swap_id)?;

        Ok(swap.hbit_redeemed.map(Into::into))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HbitRefunded {
    pub transaction: comit::transaction::Bitcoin,
}

impl From<HbitRefunded> for hbit::Refunded {
    fn from(event: HbitRefunded) -> Self {
        hbit::Refunded {
            transaction: event.transaction,
        }
    }
}

impl From<hbit::Refunded> for HbitRefunded {
    fn from(event: hbit::Refunded) -> Self {
        HbitRefunded {
            transaction: event.transaction,
        }
    }
}

#[async_trait::async_trait]
impl Save<hbit::Refunded> for Database {
    async fn save(&self, event: hbit::Refunded, swap_id: SwapId) -> anyhow::Result<()> {
        let stored_swap = self.get_swap(&swap_id)?;
        match stored_swap.hbit_refunded {
            Some(_) => Err(anyhow!("Hbit Refunded event is already stored")),
            None => {
                let key = serialize(&swap_id)?;

                let mut swap = stored_swap.clone();
                swap.hbit_refunded = Some(event.into());

                let old_value =
                    serialize(&stored_swap).context("Could not serialize old swap value")?;
                let new_value = serialize(&swap).context("Could not serialize new swap value")?;

                self.db
                    .compare_and_swap(key, Some(old_value), Some(new_value))
                    .context("Could not write in the DB")?
                    .context("Stored swap somehow changed, aborting saving")?;

                self.db
                    .flush_async()
                    .await
                    .map(|_| ())
                    .context("Could not flush db")
            }
        }
    }
}

impl Load<hbit::Refunded> for Database {
    fn load(&self, swap_id: SwapId) -> anyhow::Result<Option<hbit::Refunded>> {
        let swap = self.get_swap(&swap_id)?;

        Ok(swap.hbit_refunded.map(Into::into))
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Params {
    pub network: ::bitcoin::Network,
    pub asset: Amount,
    pub redeem_identity: identity::Bitcoin,
    pub refund_identity: identity::Bitcoin,
    pub expiry: Timestamp,
    pub secret_hash: SecretHash,
    pub transient_sk: secp256k1::SecretKey,
}

impl From<Params> for hbit::Params {
    fn from(params: Params) -> Self {
        let Params {
            network,
            asset,
            redeem_identity,
            refund_identity,
            expiry,
            secret_hash,
            transient_sk,
        } = params;

        hbit::Params {
            shared: hbit::SharedParams {
                network: network.into(),
                asset: asset.into(),
                redeem_identity,
                refund_identity,
                expiry,
                secret_hash,
            },
            transient_sk,
        }
    }
}

impl From<hbit::Params> for Params {
    fn from(params: hbit::Params) -> Self {
        Params {
            network: params.shared.network.into(),
            asset: params.shared.asset.into(),
            redeem_identity: params.shared.redeem_identity,
            refund_identity: params.shared.refund_identity,
            expiry: params.shared.expiry,
            secret_hash: params.shared.secret_hash,
            transient_sk: params.transient_sk,
        }
    }
}

#[cfg(test)]
impl crate::StaticStub for Params {
    fn static_stub() -> Self {
        use std::str::FromStr;

        Params {
            network: ::bitcoin::Network::Regtest,
            asset: Amount::from(comit::asset::Bitcoin::from_sat(123456789)),
            redeem_identity: comit::bitcoin::PublicKey::from_str(
                "039b6347398505f5ec93826dc61c19f47c66c0283ee9be980e29ce325a0f4679ef",
            )
            .unwrap(),
            refund_identity: comit::bitcoin::PublicKey::from_str(
                "032e58afe51f9ed8ad3cc7897f634d881fdbe49a81564629ded8156bebd2ffd1af",
            )
            .unwrap(),
            expiry: 12345678.into(),
            secret_hash: SecretHash::new(Secret::from(*b"hello world, you are beautiful!!")),
            transient_sk: secp256k1::SecretKey::from_str(
                "01010101010101010001020304050607ffff0000ffff00006363636363636363",
            )
            .unwrap(),
        }
    }
}

// TODO: deserialisation/serialisation round
// TODO: proptests everywhere
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        database::{Database, Swap},
        swap::SwapKind,
        StaticStub,
    };

    fn bitcoin_transaction() -> ::bitcoin::Transaction {
        ::bitcoin::Transaction {
            version: 1,
            lock_time: 0,
            input: vec![::bitcoin::TxIn {
                previous_output: Default::default(),
                script_sig: Default::default(),
                sequence: 0,
                witness: vec![],
            }],
            output: vec![::bitcoin::TxOut {
                value: 0,
                script_pubkey: Default::default(),
            }],
        }
    }

    #[tokio::test]
    async fn save_and_load_hbit_funded() {
        let db = Database::new_test().unwrap();
        let asset = comit::asset::Bitcoin::from_sat(123456);
        let location = comit::htlc_location::Bitcoin::default();
        let swap = Swap::static_stub();
        let swap_id = SwapId::default();

        let swap_kind = SwapKind::from((swap, swap_id));

        db.insert_swap(swap_kind).await.unwrap();

        let funded = hbit::Funded { asset, location };
        db.save(funded, swap_id).await.unwrap();

        let stored_funded: hbit::Funded = db
            .load(swap_id)
            .expect("No error loading")
            .expect("found the event");

        assert_eq!(stored_funded.asset, asset);
        assert_eq!(stored_funded.location, location);
    }

    #[tokio::test]
    async fn save_and_load_hbit_redeemed() {
        let db = Database::new_test().unwrap();
        let transaction = bitcoin_transaction();
        let secret = Secret::from_vec(b"are those thirty-two bytes? Hum.").unwrap();
        let swap = Swap::static_stub();
        let swap_id = SwapId::default();

        let swap_kind = SwapKind::from((swap, swap_id));

        db.insert_swap(swap_kind).await.unwrap();

        let event = hbit::Redeemed {
            transaction: transaction.clone(),
            secret,
        };
        db.save(event, swap_id).await.unwrap();

        let stored_event: hbit::Redeemed = db
            .load(swap_id)
            .expect("No error loading")
            .expect("found the event");

        assert_eq!(stored_event.transaction, transaction);
        assert_eq!(stored_event.secret, secret);
    }

    #[tokio::test]
    async fn save_and_load_hbit_refunded() {
        let db = Database::new_test().unwrap();
        let transaction = bitcoin_transaction();
        let swap = Swap::static_stub();
        let swap_id = SwapId::default();

        let swap_kind = SwapKind::from((swap, swap_id));

        db.insert_swap(swap_kind).await.unwrap();

        let event = hbit::Refunded {
            transaction: transaction.clone(),
        };
        db.save(event, swap_id).await.unwrap();

        let stored_event: hbit::Refunded = db
            .load(swap_id)
            .expect("No error loading")
            .expect("found the event");

        assert_eq!(stored_event.transaction, transaction);
    }
}
