use self::{
    hbit::{HbitFunded, HbitRedeemed, HbitRefunded},
    herc20::{Herc20Deployed, Herc20Funded, Herc20Redeemed, Herc20Refunded},
};
use crate::{swap, swap::SwapKind, SwapId};
use anyhow::{anyhow, Context};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

mod hbit;
mod herc20;

pub trait Load<T>: Send + Sync + 'static {
    fn load(&self, swap_id: SwapId) -> anyhow::Result<Option<T>>;
}

pub trait Save<T>: Send + Sync + 'static {
    fn save(&self, elem: T, swap_id: SwapId) -> anyhow::Result<()>;
}

#[derive(Debug)]
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

    pub fn insert(&self, swap: SwapKind) -> anyhow::Result<()> {
        let swap_id = match swap {
            SwapKind::HbitHerc20(ref swap_params) => swap_params.swap_id,
            SwapKind::Herc20Hbit(ref swap_params) => swap_params.swap_id,
        };

        let stored_swap = self.get(&swap_id);

        match stored_swap {
            Ok(_) => Err(anyhow!("Swap is already stored")),
            Err(_) => {
                let swap: Swap = swap.into();
                let new_value =
                    serde_json::to_vec(&swap).context("Could not serialize new swap value")?;

                self.db
                    .compare_and_swap(swap_id.as_bytes(), Option::<Vec<u8>>::None, Some(new_value))
                    .context("Could not write in the DB")?
                    .context("Stored swap somehow changed, aborting saving")
            }
        }
    }

    pub fn load_all(&self) -> anyhow::Result<Vec<SwapKind>> {
        todo!()
    }

    pub fn remove(&self, swap_id: &SwapId) -> anyhow::Result<()> {
        let key = swap_id.as_bytes();

        self.db
            .remove(key)
            .context(format!("Could not delete swap {}", swap_id))
            .map(|_| ())
    }

    // TODO: Add versioning to the data
    fn _insert(&self, swap_id: &SwapId, swap: &Swap) -> anyhow::Result<()> {
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
    pub kind: Kind,
    pub hbit_params: hbit::Params,
    pub herc20_params: herc20::Params,
    pub secret_hash: comit::SecretHash,
    pub start_of_swap: DateTime<Local>,
    pub hbit_funded: Option<HbitFunded>,
    pub hbit_redeemed: Option<HbitRedeemed>,
    pub hbit_refunded: Option<HbitRefunded>,
    pub herc20_deployed: Option<Herc20Deployed>,
    pub herc20_funded: Option<Herc20Funded>,
    pub herc20_redeemed: Option<Herc20Redeemed>,
    pub herc20_refunded: Option<Herc20Refunded>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
enum Kind {
    HbitHerc20,
    Herc20Hbit,
}

#[cfg(test)]
impl Default for Swap {
    fn default() -> Self {
        use std::str::FromStr;

        Swap {
            kind: Kind::HbitHerc20,
            hbit_params: Default::default(),
            herc20_params: Default::default(),
            secret_hash: comit::SecretHash::new(
                comit::Secret::from_str(
                    "aa68d627971643a6f97f27c58957826fcba853ec2077fd10ec6b93d8e61deb4c",
                )
                .unwrap(),
            ),
            start_of_swap: Local::now(),
            hbit_funded: None,
            hbit_redeemed: None,
            hbit_refunded: None,
            herc20_deployed: None,
            herc20_funded: None,
            herc20_redeemed: None,
            herc20_refunded: None,
        }
    }
}

// TODO: Is that needed?
impl From<(Swap, SwapId)> for SwapKind {
    fn from(swap_data: (Swap, SwapId)) -> Self {
        let (swap, swap_id) = swap_data;

        let Swap {
            kind,
            hbit_params,
            herc20_params,
            secret_hash,
            start_of_swap,
            ..
        } = swap;

        let swap = swap::SwapParams {
            hbit_params: hbit_params.into(),
            herc20_params: herc20_params.into(),
            secret_hash,
            start_of_swap: start_of_swap.naive_local(),
            swap_id,
        };

        match kind {
            Kind::HbitHerc20 => SwapKind::HbitHerc20(swap),
            Kind::Herc20Hbit => SwapKind::Herc20Hbit(swap),
        }
    }
}

impl From<SwapKind> for Swap {
    fn from(_: SwapKind) -> Self {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_and_retrieve_swaps() {
        let db = Database::new_test().unwrap();

        let swap_1 = SwapKind::HbitHerc20(swap::SwapParams::default());
        let swap_2 = SwapKind::Herc20Hbit(swap::SwapParams::default());

        db.insert(swap_1.clone()).unwrap();
        db.insert(swap_2.clone()).unwrap();

        let stored_swaps = db.load_all().unwrap();

        assert_eq!(stored_swaps, vec![swap_1, swap_2]);
    }

    #[test]
    fn save_and_delete_correct_swap() {
        let db = Database::new_test().unwrap();
        let swap_1 = swap::SwapParams::default();
        let swap_id_1 = swap_1.swap_id;

        let swap_1 = SwapKind::HbitHerc20(swap_1);
        let swap_2 = SwapKind::Herc20Hbit(swap::SwapParams::default());

        db.insert(swap_1).unwrap();
        db.insert(swap_2.clone()).unwrap();

        db.remove(&swap_id_1).unwrap();

        let stored_swaps = db.load_all().unwrap();

        assert_eq!(stored_swaps, vec![swap_2]);
    }
}
