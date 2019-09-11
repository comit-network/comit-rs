mod models;
mod schema;

pub use crate::db::models::Metadata;

use crate::{
    db::models::NewMetadata,
    swap_protocols::{
        metadata_store::{self, AssetKind, Error, LedgerKind, MetadataStore, Role},
        SwapId,
    },
};
use diesel::{self, prelude::*, sqlite::SqliteConnection};
use libp2p::PeerId;
use migrations_internals;
use std::{
    convert::TryFrom,
    fs::File,
    io::stdout,
    path::{Path, PathBuf},
    str::FromStr,
};

/// This module provides an Sqlite backed persistent MatadataStore.

#[derive(Debug)]
pub struct SqliteMetadataStore {
    db: PathBuf,
}

impl SqliteMetadataStore {
    /// Return a handle that can be used to access the database.
    ///
    /// When this returns an Sqlite database exists at 'db' and connection to
    /// the database has been verified.
    pub fn new(db: Option<PathBuf>) -> Result<SqliteMetadataStore, Error> {
        let db = db
            .or(default_db_path())
            .ok_or_else(|| Error::Path("Failed to get database path".to_string()))?;

        create_database_if_needed(&db)?;
        log::info!("Sqlite database file: {}", db.display());

        let migrations = default_migrations_dir()
            .ok_or_else(|| Error::Path("Failed to get migrations directory".to_string()))?;

        create_migrations_directory_if_needed(&migrations)?;
        run_migrations(&db, &migrations)?;

        Ok(SqliteMetadataStore { db })
    }
}

impl MetadataStore for SqliteMetadataStore {
    fn insert<M: Into<metadata_store::Metadata>>(
        &self,
        metadata: M,
    ) -> Result<(), metadata_store::Error> {
        use schema::metadatas;

        let md = Metadata::new(metadata.into());
        let new = NewMetadata::new(&md);

        let conn = establish_connection(&self.db)?;
        diesel::insert_into(metadatas::table)
            .values(new)
            .execute(&conn)
            .map(|_| ()) // Drops the '1' for successful row insert.
            .map_err(|err| Error::Insert(err.to_string()))
    }

    fn get(&self, key: SwapId) -> Result<Option<metadata_store::Metadata>, Error> {
        use self::schema::metadatas::dsl::*;

        let key = key.to_string();
        let conn = establish_connection(&self.db)?;
        let records = metadatas
            .filter(swap_id.eq(key))
            .load::<Metadata>(&conn)
            .map_err(|err| Error::Load(err.to_string()))?;

        if records.len() == 0 {
            return Ok(None);
        }

        let metadata = metadata_store::Metadata::try_from(records[0].clone())?;

        Ok(Some(metadata))
    }

    fn all(&self) -> Result<Vec<metadata_store::Metadata>, Error> {
        // Imports a bunch of aliases so that we can say metadatas instead of
        // metadatas::table. It's useful when we're only dealing with a single table.
        use self::schema::metadatas::dsl::*;

        let conn = establish_connection(&self.db)?;
        let records = metadatas
            .load::<Metadata>(&conn)
            .map_err(|err| Error::Load(err.to_string()))?;

        let v: Result<Vec<_>, _> = records
            .iter()
            .map(move |m| metadata_store::Metadata::try_from(m.clone()))
            .collect();

        v
    }
}

pub fn default_db_path() -> Option<PathBuf> {
    crate::data_dir().map(|dir| Path::join(&dir, "cnd.sqlite"))
}

pub fn default_migrations_dir() -> Option<PathBuf> {
    crate::data_dir().map(|dir| Path::join(&dir, "migrations"))
}

fn run_migrations(db: &Path, migrations: &Path) -> Result<(), Error> {
    let conn = establish_connection(&db)?;
    migrations_internals::run_pending_migrations_in_directory(&conn, migrations, &mut stdout())
        .map_err(|err| Error::Init(err.to_string()))?;

    Ok(())
}

fn create_database_if_needed(db: &Path) -> Result<(), Error> {
    if !db.exists() {
        log::info!("Creating database: {}", db.display());
        let _file = File::create(db).map_err(|err| Error::Path(err.to_string()))?;
    }
    Ok(())
}

fn create_migrations_directory_if_needed(dir: &Path) -> Result<(), Error> {
    if !dir.exists() {
        let _dir = std::fs::create_dir(dir).map_err(|err| Error::Path(err.to_string()))?;
    }
    Ok(())
}

fn establish_connection(db: &Path) -> Result<SqliteConnection, Error> {
    let database_url = db
        .to_str()
        .ok_or_else(|| Error::Path(format!("Database path invalid: {}", db.to_string_lossy())))?;

    SqliteConnection::establish(&database_url).map_err(|err| Error::Connect(err.to_string()))
}

impl TryFrom<Metadata> for metadata_store::Metadata {
    type Error = metadata_store::Error;

    fn try_from(md: Metadata) -> Result<Self, Self::Error> {
        let swap_id = SwapId::from_str(md.swap_id.as_str())?;
        let alpha_ledger = LedgerKind::from_str(md.alpha_ledger.as_str())?;
        let beta_ledger = LedgerKind::from_str(md.beta_ledger.as_str())?;
        let alpha_asset = AssetKind::from_str(md.alpha_asset.as_str())?;
        let beta_asset = AssetKind::from_str(md.beta_asset.as_str())?;
        let role = Role::from_str(md.role.as_str())?;
        let counterparty = PeerId::from_str(md.counterparty.as_str()).unwrap();

        Ok(metadata_store::Metadata {
            swap_id,
            alpha_ledger,
            beta_ledger,
            alpha_asset,
            beta_asset,
            role,
            counterparty,
        })
    }
}
