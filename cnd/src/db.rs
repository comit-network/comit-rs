mod created_swap;
mod in_progress_swap;
#[cfg(test)]
mod integration_tests;
mod load_swaps;
mod rfc003_schema;
mod save;
mod schema;
pub mod tables;
mod wrapper_types;
#[macro_use]
mod swap;
#[macro_use]
mod swap_types;
#[macro_use]
pub mod with_swap_types;
embed_migrations!("./migrations");

pub use self::{
    load_swaps::{AcceptedSwap, LoadAcceptedSwap},
    save::*,
    swap::*,
    swap_types::*,
};

use crate::{
    db::wrapper_types::custom_sql_types::Text,
    swap_protocols::{
        halight, han,
        ledger::{ethereum::ChainId, Ethereum},
        rfc003::{create_swap::HtlcParams, SecretHash, SwapId},
        LocalSwapId, Role,
    },
};
use async_trait::async_trait;
use blockchain_contracts::ethereum::rfc003::ether_htlc::EtherHtlc;
use diesel::{self, prelude::*, sqlite::SqliteConnection};
use libp2p::PeerId;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::Mutex;

/// This module provides persistent storage by way of Sqlite.

/// Save date to the database.
#[async_trait]
pub trait Save<T>: Send + Sync + 'static {
    async fn save(&self, swap: T) -> anyhow::Result<()>;
}

/// Load data from the database.
#[async_trait]
pub trait Load<T>: Send + Sync + 'static {
    async fn load(&self, swap_id: LocalSwapId) -> anyhow::Result<Option<T>>;
}

#[derive(Clone, derivative::Derivative)]
#[derivative(Debug)]
pub struct Sqlite {
    #[derivative(Debug = "ignore")]
    connection: Arc<Mutex<SqliteConnection>>,
}

impl Sqlite {
    /// Return a handle that can be used to access the database.
    ///
    /// When this returns, an Sqlite database file 'cnd.sql' exists in 'dir', a
    /// successful connection to the database has been made, and the database
    /// migrations have been run.
    pub fn new_in_dir<D>(dir: D) -> anyhow::Result<Self>
    where
        D: AsRef<OsStr>,
    {
        let dir = Path::new(&dir);
        let path = db_path_from_dir(dir);
        Sqlite::new(&path)
    }

    /// Return a handle that can be used to access the database.
    ///
    /// Reads or creates an SQLite database file at 'file'.  When this returns
    /// an Sqlite database exists, a successful connection to the database has
    /// been made, and the database migrations have been run.
    pub fn new(file: &Path) -> anyhow::Result<Self> {
        ensure_folder_tree_exists(file)?;

        let connection = SqliteConnection::establish(&format!("file:{}", file.display()))?;
        embedded_migrations::run(&connection)?;

        tracing::info!("SQLite database file: {}", file.display());

        Ok(Sqlite {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    async fn do_in_transaction<F, T, E>(&self, f: F) -> Result<T, E>
    where
        F: FnOnce(&SqliteConnection) -> Result<T, E>,
        E: From<diesel::result::Error>,
    {
        let guard = self.connection.lock().await;
        let connection = &*guard;

        let result = connection.transaction(|| f(&connection))?;

        Ok(result)
    }

    async fn rfc003_role(&self, key: &SwapId) -> anyhow::Result<Role> {
        use self::rfc003_schema::rfc003_swaps as swaps;

        let record: QueryableSwapRole = self
            .do_in_transaction(|connection| {
                let key = Text(key);

                swaps::table
                    .filter(swaps::swap_id.eq(key))
                    .select((swaps::swap_id, swaps::role))
                    .first(connection)
                    .optional()
            })
            .await?
            .ok_or(Error::SwapNotFound)?;

        Ok(*record.role)
    }
}

// Construct an absolute path to the database file using 'dir' as the base.
fn db_path_from_dir(dir: &Path) -> PathBuf {
    let path = dir.to_path_buf();
    path.join("cnd.sqlite")
}

fn ensure_folder_tree_exists(path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    Ok(())
}

#[derive(Queryable, Debug, Clone, PartialEq)]
struct QueryableSwapRole {
    pub swap_id: Text<SwapId>,
    pub role: Text<Role>,
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum Error {
    #[error("swap not found")]
    SwapNotFound,
    #[error("peer id not found")]
    PeerIdNotFound,
    #[error("identity is not set")]
    IdentityNotSet,
}

/// Data required to create a swap.
///
/// 'create' a swap is defined as the process of initiating a swap within `cnd`.
/// The data required to do so is assumed to have been negotiated between the
/// two parties prior to each creating the swap. This struct can be saved into
/// the database.
#[derive(Debug, Clone, PartialEq)]
pub struct CreatedSwap<A, B> {
    /// Node specific swap identifier.
    pub swap_id: LocalSwapId,
    /// The parameters used on the alpha ledger.
    pub alpha: A,
    /// The parameters used on the beta ledger.
    pub beta: B,
    /// Peer ID of the swap counterparty.
    pub peer: PeerId,
    /// The address hint of the swap counterparty, only relevant to the party
    /// that starts the communication.
    pub address_hint: Option<libp2p::Multiaddr>,
    /// Role of the node in this swap, Alice or Bob.
    pub role: Role,
}

/// An 'in progress' swap is a swap that is currently in progress, i.e., the
/// swap was 'created' by a POST on the REST API, 'finalized' during the
/// communication protocols, and is now 'in progress'.  This data structure can
/// be retrieved from the database.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InProgressSwap<A, B> {
    pub swap_id: LocalSwapId,
    pub secret_hash: SecretHash,
    pub role: Role,
    pub alpha: A,
    pub beta: B,
}

impl From<&InProgressSwap<han::InProgressSwap, halight::InProgressSwap>> for EtherHtlc {
    fn from(swap: &InProgressSwap<han::InProgressSwap, halight::InProgressSwap>) -> Self {
        HtlcParams {
            asset: swap.alpha.asset.clone(),
            ledger: Ethereum::new(ChainId::regtest()),
            redeem_identity: swap.alpha.redeem_identity,
            refund_identity: swap.alpha.refund_identity,
            expiry: swap.alpha.expiry,
            secret_hash: swap.secret_hash,
        }
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectral::prelude::*;
    use std::path::PathBuf;

    fn temp_db() -> PathBuf {
        let temp_file = tempfile::Builder::new()
            .suffix(".sqlite")
            .tempfile()
            .unwrap();

        temp_file.into_temp_path().to_path_buf()
    }

    #[test]
    fn can_create_a_new_temp_db() {
        let path = temp_db();

        let db = Sqlite::new(&path);

        assert_that(&db).is_ok();
    }

    #[test]
    fn given_no_database_exists_calling_new_creates_it() {
        let path = temp_db();
        // validate assumptions: the db does not exist yet
        assert_that(&path.as_path()).does_not_exist();

        let db = Sqlite::new(&path);

        assert_that(&db).is_ok();
        assert_that(&path.as_path()).exists();
    }

    #[test]
    fn given_db_in_non_existing_directory_tree_calling_new_creates_it() {
        let tempfile = tempfile::tempdir().unwrap();
        let mut path = PathBuf::new();

        path.push(tempfile);
        path.push("some_folder");
        path.push("i_dont_exist");
        path.push("database.sqlite");

        // validate assumptions:
        // 1. the db does not exist yet
        // 2. the parent folder does not exist yet
        assert_that(&path).does_not_exist();
        assert_that(&path.parent()).is_some().does_not_exist();

        let db = Sqlite::new(&path);

        assert_that(&db).is_ok();
        assert_that(&path).exists();
    }
}
