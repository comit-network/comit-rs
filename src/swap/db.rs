use crate::{
    swap::{hbit, herc20, SwapKind},
    SwapId,
};
use anyhow::{anyhow, Context};
use comit::{
    asset::Erc20,
    ethereum::{self, Hash, Transaction, U256},
    Secret,
};
use serde::{Deserialize, Serialize};
use serde_hex::{SerHexSeq, StrictPfx};

#[async_trait::async_trait]
pub trait Load<T>: Send + Sync + 'static {
    async fn load(&self, swap_id: SwapId) -> anyhow::Result<Option<T>>;
}

#[async_trait::async_trait]
pub trait Save<T>: Send + Sync + 'static {
    async fn save(&self, elem: T, swap_id: SwapId) -> anyhow::Result<()>;
}

#[derive(Debug, Clone, Copy)]
pub struct Created;

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

    pub fn load_all(&self) -> anyhow::Result<Vec<SwapKind>> {
        todo!()
    }

    pub fn delete(&self, swap_id: &SwapId) -> anyhow::Result<()> {
        let key = swap_id.as_bytes();

        self.db
            .remove(key)
            .context(format!("Could not delete swap {}", swap_id))
            .map(|_| ())
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
    pub herc20_deployed: Option<Herc20Deployed>,
    pub herc20_funded: Option<Herc20Funded>,
    pub herc20_redeemed: Option<Herc20Redeemed>,
    pub herc20_refunded: Option<Herc20Refunded>,
}

impl Default for Swap {
    fn default() -> Self {
        Swap {
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

// Kind of bending the arm of the trait
#[async_trait::async_trait]
impl Save<Created> for Database {
    async fn save(&self, _event: Created, swap_id: SwapId) -> anyhow::Result<()> {
        let stored_swap = self.get(&swap_id);

        match stored_swap {
            Ok(_) => Err(anyhow!("Swap is already stored")),
            Err(_) => {
                let swap = Swap::default();
                let new_value =
                    serde_json::to_vec(&swap).context("Could not serialize new swap value")?;

                self.db
                    .compare_and_swap(swap_id.as_bytes(), Option::<Vec<u8>>::None, Some(new_value))
                    .context("Could not write in the DB")?
                    .context("Stored swap somehow changed, aborting saving")
            }
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
impl Save<SwapKind> for Database {
    async fn save(&self, _elem: SwapKind, _swap_id: SwapId) -> anyhow::Result<()> {
        todo!()
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

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Herc20Deployed {
    pub transaction: EthereumTransaction,
    pub location: comit::htlc_location::Ethereum,
}

impl From<Herc20Deployed> for herc20::Deployed {
    fn from(event: Herc20Deployed) -> Self {
        herc20::Deployed {
            transaction: event.transaction.into(),
            location: event.location,
        }
    }
}

impl From<herc20::Deployed> for Herc20Deployed {
    fn from(event: herc20::Deployed) -> Self {
        Herc20Deployed {
            transaction: event.transaction.into(),
            location: event.location,
        }
    }
}

#[async_trait::async_trait]
impl Save<herc20::Deployed> for Database {
    async fn save(&self, event: herc20::Deployed, swap_id: SwapId) -> anyhow::Result<()> {
        let stored_swap = self.get(&swap_id)?;

        match stored_swap.herc20_deployed {
            Some(_) => Err(anyhow!("Herc20 Deployed event is already stored")),
            None => {
                let mut swap = stored_swap.clone();
                swap.herc20_deployed = Some(event.into());

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
impl Load<herc20::Deployed> for Database {
    async fn load(&self, swap_id: SwapId) -> anyhow::Result<Option<herc20::Deployed>> {
        let swap = self.get(&swap_id)?;

        Ok(swap.herc20_deployed.map(Into::into))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Herc20Funded {
    pub transaction: EthereumTransaction,
    pub asset: Erc20Asset,
}

impl From<Herc20Funded> for herc20::Funded {
    fn from(event: Herc20Funded) -> Self {
        herc20::Funded {
            transaction: event.transaction.into(),
            asset: event.asset.into(),
        }
    }
}

impl From<herc20::Funded> for Herc20Funded {
    fn from(event: herc20::Funded) -> Self {
        Herc20Funded {
            transaction: event.transaction.into(),
            asset: event.asset.into(),
        }
    }
}

#[async_trait::async_trait]
impl Save<herc20::Funded> for Database {
    async fn save(&self, event: herc20::Funded, swap_id: SwapId) -> anyhow::Result<()> {
        let stored_swap = self.get(&swap_id)?;

        match stored_swap.herc20_funded {
            Some(_) => Err(anyhow!("Herc20 Funded event is already stored")),
            None => {
                let mut swap = stored_swap.clone();
                swap.herc20_funded = Some(event.into());

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
impl Load<herc20::Funded> for Database {
    async fn load(&self, swap_id: SwapId) -> anyhow::Result<Option<herc20::Funded>> {
        let swap = self.get(&swap_id)?;

        Ok(swap.herc20_funded.map(Into::into))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Herc20Redeemed {
    pub transaction: EthereumTransaction,
    pub secret: Secret,
}

impl From<Herc20Redeemed> for herc20::Redeemed {
    fn from(event: Herc20Redeemed) -> Self {
        herc20::Redeemed {
            transaction: event.transaction.into(),
            secret: event.secret,
        }
    }
}

impl From<herc20::Redeemed> for Herc20Redeemed {
    fn from(event: herc20::Redeemed) -> Self {
        Herc20Redeemed {
            transaction: event.transaction.into(),
            secret: event.secret,
        }
    }
}

#[async_trait::async_trait]
impl Save<herc20::Redeemed> for Database {
    async fn save(&self, event: herc20::Redeemed, swap_id: SwapId) -> anyhow::Result<()> {
        let stored_swap = self.get(&swap_id)?;

        match stored_swap.herc20_redeemed {
            Some(_) => Err(anyhow!("Herc20 Redeemed event is already stored")),
            None => {
                let mut swap = stored_swap.clone();
                swap.herc20_redeemed = Some(event.into());

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
impl Load<herc20::Redeemed> for Database {
    async fn load(&self, swap_id: SwapId) -> anyhow::Result<Option<herc20::Redeemed>> {
        let swap = self.get(&swap_id)?;

        Ok(swap.herc20_redeemed.map(Into::into))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Herc20Refunded {
    pub transaction: EthereumTransaction,
}

impl From<Herc20Refunded> for herc20::Refunded {
    fn from(event: Herc20Refunded) -> Self {
        herc20::Refunded {
            transaction: event.transaction.into(),
        }
    }
}

impl From<herc20::Refunded> for Herc20Refunded {
    fn from(event: herc20::Refunded) -> Self {
        Herc20Refunded {
            transaction: event.transaction.into(),
        }
    }
}

#[async_trait::async_trait]
impl Save<herc20::Refunded> for Database {
    async fn save(&self, event: herc20::Refunded, swap_id: SwapId) -> anyhow::Result<()> {
        let stored_swap = self.get(&swap_id)?;

        match stored_swap.herc20_refunded {
            Some(_) => Err(anyhow!("Herc20 Refunded event is already stored")),
            None => {
                let mut swap = stored_swap.clone();
                swap.herc20_refunded = Some(event.into());

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
impl Load<herc20::Refunded> for Database {
    async fn load(&self, swap_id: SwapId) -> anyhow::Result<Option<herc20::Refunded>> {
        let swap = self.get(&swap_id)?;

        Ok(swap.herc20_refunded.map(Into::into))
    }
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct EthereumTransaction {
    pub hash: Hash,
    pub to: Option<ethereum::Address>,
    pub value: U256,
    #[serde(with = "SerHexSeq::<StrictPfx>")]
    pub input: Vec<u8>,
}

impl From<EthereumTransaction> for ethereum::Transaction {
    fn from(transaction: EthereumTransaction) -> Self {
        ethereum::Transaction {
            hash: transaction.hash,
            to: transaction.to,
            value: transaction.value,
            input: transaction.input,
        }
    }
}

impl From<ethereum::Transaction> for EthereumTransaction {
    fn from(transaction: Transaction) -> Self {
        EthereumTransaction {
            hash: transaction.hash,
            to: transaction.to,
            value: transaction.value,
            input: transaction.input,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Erc20Asset {
    pub token_contract: ethereum::Address,
    pub quantity: comit::asset::Erc20Quantity,
}

impl From<Erc20Asset> for comit::asset::Erc20 {
    fn from(asset: Erc20Asset) -> Self {
        comit::asset::Erc20 {
            token_contract: asset.token_contract,
            quantity: asset.quantity,
        }
    }
}

impl From<comit::asset::Erc20> for Erc20Asset {
    fn from(asset: Erc20) -> Self {
        Erc20Asset {
            token_contract: asset.token_contract,
            quantity: asset.quantity,
        }
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
        let secret = Secret::from_vec(b"are those thirty-two bytes? Hum.").unwrap();
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

    #[tokio::test]
    async fn save_and_load_herc20_deployed() {
        let db = Database::new_test().unwrap();
        let swap = Swap::default();
        let swap_id = SwapId::default();
        let transaction = comit::transaction::Ethereum::default();
        let location = comit::htlc_location::Ethereum::random();

        db.insert(&swap_id, &swap).unwrap();

        let event = herc20::Deployed {
            transaction: transaction.clone(),
            location,
        };
        db.save(event, swap_id).await.unwrap();

        let stored_event: herc20::Deployed = db
            .load(swap_id)
            .await
            .expect("No error loading")
            .expect("found the event");

        assert_eq!(stored_event.transaction, transaction);
        assert_eq!(stored_event.location, location);
    }

    #[tokio::test]
    async fn save_and_load_herc20_funded() {
        let db = Database::new_test().unwrap();
        let swap = Swap::default();
        let swap_id = SwapId::default();
        let transaction = comit::transaction::Ethereum::default();
        let asset = comit::asset::Erc20::new(
            ethereum::Address::random(),
            comit::asset::Erc20Quantity::from_wei_dec_str("123456789012345678").unwrap(),
        );

        db.insert(&swap_id, &swap).unwrap();

        let event = herc20::Funded {
            transaction: transaction.clone(),
            asset: asset.clone(),
        };
        db.save(event, swap_id).await.unwrap();

        let stored_event: herc20::Funded = db
            .load(swap_id)
            .await
            .expect("No error loading")
            .expect("found the event");

        assert_eq!(stored_event.transaction, transaction);
        assert_eq!(stored_event.asset, asset);
    }

    #[tokio::test]
    async fn save_and_load_herc20_redeemed() {
        let db = Database::new_test().unwrap();
        let swap = Swap::default();
        let swap_id = SwapId::default();
        let transaction = comit::transaction::Ethereum::default();
        let secret = Secret::from_vec(b"are those thirty-two bytes? Hum.").unwrap();

        db.insert(&swap_id, &swap).unwrap();

        let event = herc20::Redeemed {
            transaction: transaction.clone(),
            secret,
        };
        db.save(event, swap_id).await.unwrap();

        let stored_event: herc20::Redeemed = db
            .load(swap_id)
            .await
            .expect("No error loading")
            .expect("found the event");

        assert_eq!(stored_event.transaction, transaction);
        assert_eq!(stored_event.secret, secret);
    }

    #[tokio::test]
    async fn save_and_load_herc20_refunded() {
        let db = Database::new_test().unwrap();
        let swap = Swap::default();
        let swap_id = SwapId::default();
        let transaction = comit::transaction::Ethereum::default();

        db.insert(&swap_id, &swap).unwrap();

        let event = herc20::Refunded {
            transaction: transaction.clone(),
        };
        db.save(event, swap_id).await.unwrap();

        let stored_event: herc20::Refunded = db
            .load(swap_id)
            .await
            .expect("No error loading")
            .expect("found the event");

        assert_eq!(stored_event.transaction, transaction);
    }
}
