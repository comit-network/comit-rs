use self::{
    hbit::{HbitFunded, HbitRedeemed, HbitRefunded},
    herc20::{Herc20Deployed, Herc20Funded, Herc20Redeemed, Herc20Refunded},
};
use crate::{network, network::ActivePeer, swap, swap::SwapKind, SwapId};
use anyhow::{anyhow, Context};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[cfg(test)]
use crate::StaticStub;
use std::{collections::HashSet, iter::FromIterator};

mod hbit;
mod herc20;

pub trait Load<T>: Send + Sync + 'static {
    fn load(&self, swap_id: SwapId) -> anyhow::Result<Option<T>>;
}

#[async_trait::async_trait]
pub trait Save<T>: Send + Sync + 'static {
    async fn save(&self, elem: T, swap_id: SwapId) -> anyhow::Result<()>;
}

#[derive(Debug)]
pub struct Database {
    db: sled::Db,
    #[cfg(test)]
    tmp_dir: tempfile::TempDir,
}

impl Database {
    const ACTIVE_PEER_KEY: &'static str = "active_peer";
    const BITCOIN_TRANSIENT_KEYS_INDEX_KEY: &'static str = "bitcoin_transient_key_index";

    #[cfg(not(test))]
    pub fn new(path: &std::path::Path) -> anyhow::Result<Self> {
        let path = path
            .to_str()
            .ok_or_else(|| anyhow!("The path is not utf-8 valid: {:?}", path))?;
        let db = sled::open(path).with_context(|| format!("Could not open the DB at {}", path))?;

        if !db.contains_key(Self::ACTIVE_PEER_KEY)? {
            let peers = Vec::<ActivePeer>::new();
            let peers = serialize(&peers)?;
            let _ = db.insert(serialize(&Self::ACTIVE_PEER_KEY)?, peers)?;
        }

        if !db.contains_key(Self::BITCOIN_TRANSIENT_KEYS_INDEX_KEY)? {
            let index = serialize(&0u32)?;
            let _ = db.insert(serialize(&Self::BITCOIN_TRANSIENT_KEYS_INDEX_KEY)?, index)?;
        }

        Ok(Database { db })
    }

    #[cfg(test)]
    pub fn new_test() -> anyhow::Result<Self> {
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let db = sled::open(tmp_dir.path())
            .with_context(|| format!("Could not open the DB at {}", tmp_dir.path().display()))?;

        let peers = Vec::<ActivePeer>::new();
        let peers = serialize(&peers)?;
        let _ = db.insert(serialize(&Self::ACTIVE_PEER_KEY)?, peers)?;

        let index = serialize(&0u32)?;
        let _ = db.insert(serialize(&Self::BITCOIN_TRANSIENT_KEYS_INDEX_KEY)?, index)?;

        Ok(Database { db, tmp_dir })
    }

    pub async fn fetch_inc_bitcoin_transient_key_index(&self) -> anyhow::Result<u32> {
        let old_value = self.db.fetch_and_update(
            serialize(&Self::BITCOIN_TRANSIENT_KEYS_INDEX_KEY)?,
            |old| match old {
                Some(bytes) => deserialize::<u32>(bytes)
                    .map_err(|err| {
                        tracing::error!(
                            "Bitcoin transient keys index is corrupted in the db: {:?}, {:#}",
                            bytes,
                            err
                        )
                    })
                    .map(|index| serialize(&(index + 1)).expect("Can always serialized a u32"))
                    .ok(),
                None => None,
            },
        )?;

        self.db
            .flush_async()
            .await
            .map(|_| ())
            .context("Could not flush db")?;

        match old_value {
            Some(index) => deserialize(&index),
            None => Err(anyhow!(
                "The Bitcoin transient keys index was not properly instantiated in the db"
            )),
        }
    }
}
/// Swap related functions
impl Database {
    // TODO: Add versioning to the data
    pub async fn insert_swap(&self, swap: SwapKind) -> anyhow::Result<()> {
        let swap_id = swap.swap_id();

        let stored_swap = self.get_swap(&swap_id);

        match stored_swap {
            Ok(_) => Err(anyhow!("Swap is already stored")),
            Err(_) => {
                let key = serialize(&swap_id)?;

                let swap: Swap = swap.into();
                let new_value = serialize(&swap).context("Could not serialize new swap value")?;

                self.db
                    .compare_and_swap(key, Option::<Vec<u8>>::None, Some(new_value))
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

    pub fn all_swaps(&self) -> anyhow::Result<Vec<SwapKind>> {
        self.db
            .iter()
            .filter_map(|item| match item {
                Ok((key, value)) => {
                    let swap_id = deserialize::<SwapId>(&key);
                    let swap = deserialize::<Swap>(&value).context("Could not deserialize swap");

                    match (swap_id, swap) {
                        (Ok(swap_id), Ok(swap)) => Some(Ok(SwapKind::from((swap, swap_id)))),
                        (Ok(_), Err(err)) => Some(Err(err)), // If the swap id deserialize, then
                        // it should be a swap
                        (..) => None, // This is not a swap item
                    }
                }
                Err(err) => Some(Err(err).context("Could not retrieve data")),
            })
            .collect()
    }

    pub async fn remove_swap(&self, swap_id: &SwapId) -> anyhow::Result<()> {
        let key = serialize(swap_id)?;

        self.db
            .remove(key)
            .with_context(|| format!("Could not delete swap {}", swap_id))
            .map(|_| ())?;

        self.db
            .flush_async()
            .await
            .map(|_| ())
            .context("Could not flush db")
    }

    fn get_swap(&self, swap_id: &SwapId) -> anyhow::Result<Swap> {
        let key = serialize(swap_id)?;

        let swap = self
            .db
            .get(&key)?
            .ok_or_else(|| anyhow!("Swap does not exists {}", swap_id))?;

        deserialize(&swap).context("Could not deserialize swap")
    }
}

/// These methods are used to prevent a peer from having more than one ongoing
/// swap with nectar An active peer refers to one that has an ongoing swap with
/// nectar.
impl Database {
    pub async fn insert_active_peer(&self, peer: ActivePeer) -> anyhow::Result<()> {
        self.modify_peers_with(|peers: &mut HashSet<ActivePeer>| peers.insert(peer.clone()))?;

        self.db
            .flush_async()
            .await
            .map(|_| ())
            .context("Could not flush db")
    }

    pub async fn remove_active_peer(&self, peer: &ActivePeer) -> anyhow::Result<()> {
        self.modify_peers_with(|peers: &mut HashSet<ActivePeer>| peers.remove(peer))?;
        self.db
            .flush_async()
            .await
            .map(|_| ())
            .context("Could not flush db")
    }

    pub fn contains_active_peer(&self, peer: &ActivePeer) -> anyhow::Result<bool> {
        let peers = self.peers()?;

        Ok(peers.contains(&peer))
    }

    fn modify_peers_with(
        &self,
        operation_fn: impl Fn(&mut HashSet<ActivePeer>) -> bool,
    ) -> anyhow::Result<()> {
        let mut peers = self.peers()?;

        operation_fn(&mut peers);

        let updated_peers = Vec::<ActivePeer>::from_iter(peers);
        let updated_peers = serialize(&updated_peers)?;

        self.db
            .insert(serialize(&Self::ACTIVE_PEER_KEY)?, updated_peers)?;

        Ok(())
    }

    fn peers(&self) -> anyhow::Result<HashSet<ActivePeer>> {
        let peers = self
            .db
            .get(serialize(&Self::ACTIVE_PEER_KEY)?)?
            .ok_or_else(|| anyhow::anyhow!("no key \"active_peer\" in db"))?;
        let peers: Vec<ActivePeer> = deserialize(&peers)?;
        let peers = HashSet::<ActivePeer>::from_iter(peers);

        Ok(peers)
    }
}

pub fn serialize<T>(t: &T) -> anyhow::Result<Vec<u8>>
where
    T: Serialize,
{
    Ok(serde_cbor::to_vec(t)?)
}

pub fn deserialize<'a, T>(v: &'a [u8]) -> anyhow::Result<T>
where
    T: Deserialize<'a>,
{
    Ok(serde_cbor::from_slice(v)?)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Swap {
    pub kind: Kind,
    pub hbit_params: hbit::Params,
    pub herc20_params: herc20::Params,
    pub secret_hash: comit::SecretHash,
    pub utc_start_of_swap: DateTime<Utc>,
    pub active_peer: network::ActivePeer,
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
impl StaticStub for Swap {
    fn static_stub() -> Self {
        use std::str::FromStr;

        Swap {
            kind: Kind::HbitHerc20,
            hbit_params: StaticStub::static_stub(),
            herc20_params: StaticStub::static_stub(),
            secret_hash: comit::SecretHash::new(
                comit::Secret::from_str(
                    "aa68d627971643a6f97f27c58957826fcba853ec2077fd10ec6b93d8e61deb4c",
                )
                .unwrap(),
            ),
            active_peer: network::ActivePeer::static_stub(),
            utc_start_of_swap: chrono::Utc::now(),
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

impl From<(Swap, SwapId)> for SwapKind {
    fn from(swap_data: (Swap, SwapId)) -> Self {
        let (swap, swap_id) = swap_data;

        let Swap {
            kind,
            hbit_params,
            herc20_params,
            secret_hash,
            utc_start_of_swap: start_of_swap,
            active_peer: taker,
            ..
        } = swap;

        let swap = swap::SwapParams {
            hbit_params: hbit_params.into(),
            herc20_params: herc20_params.into(),
            secret_hash,
            start_of_swap,
            swap_id,
            taker,
        };

        match kind {
            Kind::HbitHerc20 => SwapKind::HbitHerc20(swap),
            Kind::Herc20Hbit => SwapKind::Herc20Hbit(swap),
        }
    }
}

impl From<SwapKind> for Swap {
    fn from(swap_kind: SwapKind) -> Self {
        let (kind, swap) = match swap_kind {
            SwapKind::HbitHerc20(swap) => (Kind::HbitHerc20, swap),
            SwapKind::Herc20Hbit(swap) => (Kind::Herc20Hbit, swap),
        };

        Swap {
            kind,
            hbit_params: swap.hbit_params.into(),
            herc20_params: swap.herc20_params.into(),
            secret_hash: swap.secret_hash,
            utc_start_of_swap: swap.start_of_swap,
            active_peer: swap.taker,
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

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::{Arbitrary, StdThreadGen};

    #[quickcheck_async::tokio]
    async fn save_and_retrieve_swaps(swap_1: SwapKind, swap_2: SwapKind) -> bool {
        let db = Database::new_test().unwrap();

        db.insert_swap(swap_1.clone()).await.unwrap();
        db.insert_swap(swap_2.clone()).await.unwrap();

        let stored_swaps = db.all_swaps().unwrap();

        assert_eq!(stored_swaps.len(), 2);
        assert!(stored_swaps.contains(&swap_1));
        assert!(stored_swaps.contains(&swap_2));

        true
    }

    #[quickcheck_async::tokio]
    async fn save_and_delete_correct_swap(swap_1: swap::SwapParams, swap_2: SwapKind) -> bool {
        let db = Database::new_test().unwrap();
        let swap_id_1 = swap_1.swap_id;

        let swap_1 = SwapKind::HbitHerc20(swap_1);

        db.insert_swap(swap_1).await.unwrap();
        db.insert_swap(swap_2.clone()).await.unwrap();

        db.remove_swap(&swap_id_1).await.unwrap();

        let stored_swaps = db.all_swaps().unwrap();

        assert_eq!(stored_swaps, vec![swap_2]);

        true
    }

    #[quickcheck_async::tokio]
    async fn taker_no_longer_has_ongoing_trade_after_removal(peer: ActivePeer) -> bool {
        let db = Database::new_test().unwrap();

        let _ = db.insert_active_peer(peer.clone()).await.unwrap();

        let res = db.contains_active_peer(&peer);
        assert!(matches!(res, Ok(true)));

        let _ = db.remove_active_peer(&peer).await.unwrap();
        let res = db.contains_active_peer(&peer);

        matches!(res, Ok(false))
    }

    #[tokio::test]
    async fn save_and_retrieve_hundred_swaps() {
        let db = Database::new_test().unwrap();

        let mut gen = StdThreadGen::new(100);
        let mut swaps = Vec::with_capacity(100);

        for _ in 0..100 {
            let swap = SwapKind::arbitrary(&mut gen);
            swaps.push(swap);
        }

        for swap in swaps.iter() {
            db.insert_swap(swap.clone()).await.unwrap();
        }

        let stored_swaps = db.all_swaps().unwrap();

        for swap in swaps.iter() {
            assert!(stored_swaps.contains(&swap))
        }
    }

    #[tokio::test]
    async fn increment_bitcoin_transient_key_index() {
        let db = Database::new_test().unwrap();

        assert_eq!(db.fetch_inc_bitcoin_transient_key_index().await.unwrap(), 0);
        assert_eq!(db.fetch_inc_bitcoin_transient_key_index().await.unwrap(), 1);
    }
}
