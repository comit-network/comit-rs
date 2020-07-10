use crate::swap::hbit;
use crate::SwapId;
use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};

#[async_trait::async_trait]
pub trait Load<T>: Send + Sync + 'static {
    async fn load(&self, swap_id: SwapId) -> anyhow::Result<Option<T>>;
}

#[async_trait::async_trait]
pub trait Save<T>: Send + Sync + 'static {
    async fn save(&self, event: T, swap_id: SwapId) -> anyhow::Result<()>;
}

pub struct Database {
    db: sled::Db,
    #[cfg(test)]
    tmp_dir: tempdir::TempDir,
}

impl Database {
    #[cfg(not(test))]
    pub fn new(path: &std::path::Path) -> anyhow::Result<Self> {
        let path = path
            .to_str()
            .ok_or_else(|| anyhow!("The path is not utf-8 valid: {:?}", path))?;
        let db = sled::open(path).context(format!("Could not open the DB at {}", path))?;
        Ok(Database { db })
    }

    #[cfg(test)]
    pub fn new_test() -> anyhow::Result<Self> {
        let tmp_dir = tempdir::TempDir::new("nectar_test").unwrap();
        let db = sled::open(tmp_dir.path()).context(format!(
            "Could not open the DB at {}",
            tmp_dir.path().display()
        ))?;

        Ok(Database { db, tmp_dir })
    }

    fn insert(&self, swap_id: &SwapId, swap: &Swap) -> anyhow::Result<()> {
        let key = swap_id.as_bytes();
        // TODO: Consider using https://github.com/3Hren/msgpack-rust instead
        let value = serde_json::to_vec(&swap)
            .context(format!("Could not serialize the swap: {:?}", swap))?;

        self.db
            .insert(&key, value)
            .context(format!("Could not insert swap {}", swap_id))?;

        Ok(())
    }

    fn get(&self, swap_id: &SwapId) -> anyhow::Result<Swap> {
        let swap = self
            .db
            .get(swap_id.as_bytes())?
            .ok_or_else(|| anyhow!("Swap does not exists {}", swap_id))?;

        serde_json::from_slice(&swap).context("Could not deserialize swap")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Swap {
    pub hbit_funded: Option<HbitFunded>,
    pub hbit_redeemed: Option<HbitRedeemed>,
    pub hbit_refunded: Option<HbitRefunded>,
}

impl Default for Swap {
    fn default() -> Self {
        Swap {
            hbit_funded: None,
            hbit_redeemed: None,
            hbit_refunded: None,
        }
    }
}

// TODO: control the serialisation
#[derive(Clone, Debug, Serialize, Deserialize)]
struct HbitFunded {
    pub asset: u64,
    pub location: ::bitcoin::OutPoint,
}

impl From<HbitFunded> for hbit::Funded {
    fn from(funded: HbitFunded) -> Self {
        hbit::Funded {
            asset: comit::asset::Bitcoin::from_sat(funded.asset),
            location: funded.location,
        }
    }
}

impl From<hbit::Funded> for HbitFunded {
    fn from(funded: hbit::Funded) -> Self {
        HbitFunded {
            asset: funded.asset.as_sat(),
            location: funded.location,
        }
    }
}

#[async_trait::async_trait]
impl Save<hbit::Funded> for Database {
    async fn save(&self, event: hbit::Funded, swap_id: SwapId) -> anyhow::Result<()> {
        let stored_swap = self.get(&swap_id)?;

        match stored_swap.hbit_funded {
            Some(_) => Err(anyhow!("Hbit Funded event is already stored")),
            None => {
                let mut swap = stored_swap.clone();
                swap.hbit_funded = Some(event.into());

                let old_value = serde_json::to_vec(&stored_swap)
                    .context("Could not serialize old swap value")?;
                let new_value =
                    serde_json::to_vec(&swap).context("Could not serialize new swap value")?;

                self.db
                    .compare_and_swap(swap_id.as_bytes(), Some(old_value), Some(new_value))
                    .context("Could not write in the DB")?
                    .context("Stored swap somehow changed, aborting saving")
            }
        }
    }
}

#[async_trait::async_trait]
impl Load<hbit::Funded> for Database {
    async fn load(&self, swap_id: SwapId) -> anyhow::Result<Option<hbit::Funded>> {
        let swap = self.get(&swap_id)?;

        Ok(swap.hbit_funded.map(Into::into))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct HbitRedeemed {
    pub transaction: comit::transaction::Bitcoin,
    pub secret: comit::Secret,
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
        let stored_swap = self.get(&swap_id)?;

        match stored_swap.hbit_redeemed {
            Some(_) => Err(anyhow!("Hbit Redeemed event is already stored")),
            None => {
                let mut swap = stored_swap.clone();
                swap.hbit_redeemed = Some(event.into());

                let old_value = serde_json::to_vec(&stored_swap)
                    .context("Could not serialize old swap value")?;
                let new_value =
                    serde_json::to_vec(&swap).context("Could not serialize new swap value")?;

                self.db
                    .compare_and_swap(swap_id.as_bytes(), Some(old_value), Some(new_value))
                    .context("Could not write in the DB")?
                    .context("Stored swap somehow changed, aborting saving")
            }
        }
    }
}

#[async_trait::async_trait]
impl Load<hbit::Redeemed> for Database {
    async fn load(&self, swap_id: SwapId) -> anyhow::Result<Option<hbit::Redeemed>> {
        let swap = self.get(&swap_id)?;

        Ok(swap.hbit_redeemed.map(Into::into))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct HbitRefunded {
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
        let stored_swap = self.get(&swap_id)?;

        match stored_swap.hbit_refunded {
            Some(_) => Err(anyhow!("Hbit Refunded event is already stored")),
            None => {
                let mut swap = stored_swap.clone();
                swap.hbit_refunded = Some(event.into());

                let old_value = serde_json::to_vec(&stored_swap)
                    .context("Could not serialize old swap value")?;
                let new_value =
                    serde_json::to_vec(&swap).context("Could not serialize new swap value")?;

                self.db
                    .compare_and_swap(swap_id.as_bytes(), Some(old_value), Some(new_value))
                    .context("Could not write in the DB")?
                    .context("Stored swap somehow changed, aborting saving")
            }
        }
    }
}

#[async_trait::async_trait]
impl Load<hbit::Refunded> for Database {
    async fn load(&self, swap_id: SwapId) -> anyhow::Result<Option<hbit::Refunded>> {
        let swap = self.get(&swap_id)?;

        Ok(swap.hbit_refunded.map(Into::into))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let swap = Swap::default();
        let swap_id = SwapId::default();

        db.insert(&swap_id, &swap).unwrap();

        let funded = hbit::Funded { asset, location };
        db.save(funded, swap_id).await.unwrap();

        let stored_funded: hbit::Funded = db
            .load(swap_id)
            .await
            .expect("No error loading")
            .expect("found the event");

        assert_eq!(stored_funded.asset, asset);
        assert_eq!(stored_funded.location, location);
    }

    #[tokio::test]
    async fn save_and_load_hbit_redeemed() {
        let db = Database::new_test().unwrap();
        let transaction = bitcoin_transaction();
        let secret = comit::Secret::from_vec(b"are those thirty-two bytes? Hum.").unwrap();
        let swap = Swap::default();
        let swap_id = SwapId::default();

        db.insert(&swap_id, &swap).unwrap();

        let event = hbit::Redeemed {
            transaction: transaction.clone(),
            secret,
        };
        db.save(event, swap_id).await.unwrap();

        let stored_event: hbit::Redeemed = db
            .load(swap_id)
            .await
            .expect("No error loading")
            .expect("found the event");

        assert_eq!(stored_event.transaction, transaction);
        assert_eq!(stored_event.secret, secret);
    }

    #[tokio::test]
    async fn save_and_load_hbit_refunded() {
        let db = Database::new_test().unwrap();
        let transaction = bitcoin_transaction();
        let swap = Swap::default();
        let swap_id = SwapId::default();

        db.insert(&swap_id, &swap).unwrap();

        let event = hbit::Refunded {
            transaction: transaction.clone(),
        };
        db.save(event, swap_id).await.unwrap();

        let stored_event: hbit::Refunded = db
            .load(swap_id)
            .await
            .expect("No error loading")
            .expect("found the event");

        assert_eq!(stored_event.transaction, transaction);
    }
}
