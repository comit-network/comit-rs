mod models;
mod schema;
embed_migrations!("./migrations");

use crate::{
    db::models::{InsertableMetadata, Metadata},
    metadata_store::{self, AssetKind, Error, LedgerKind, MetadataStore, Role},
};
use comit::SwapId;
use diesel::{self, prelude::*, sqlite::SqliteConnection};
use libp2p::PeerId;
use std::{
    convert::TryFrom,
    fs::File,
    path::{Path, PathBuf},
    str::FromStr,
};

/// This module provides an Sqlite backed persistent MetadataStore.

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
            .or_else(default_db_path)
            .ok_or_else(|| Error::Path("Failed to get database path".to_string()))?;

        create_database_if_needed(&db)?;

        let conn = establish_connection(&db)?;

        embedded_migrations::run(&conn)
            .map_err(|_| Error::Init("Error while running migrations".to_owned()))?;

        Ok(SqliteMetadataStore { db })
    }
}

impl MetadataStore for SqliteMetadataStore {
    fn get(&self, key: SwapId) -> Result<Option<Metadata>, Error> {
        // Imports aliases so we can refer to the table and table fields.
        use self::schema::metadatas::dsl::*;

        let key = key.to_string();
        let conn = establish_connection(&self.db)?;

        metadatas
            .filter(swap_id.eq(key))
            .first(&conn)
            .optional()
            .map_err(|err| Error::Load(err.to_string()))?
            .map(|m: Metadata| Metadata::try_from(m.clone()))
            .transpose()
    }

    fn insert(&self, metadata: metadata_store::Metadata) -> Result<(), Error> {
        let md = Metadata::new(metadata);
        let new = InsertableMetadata::new(&md);

        let conn = establish_connection(&self.db)?;
        diesel::insert_into(schema::metadatas::table)
            .values(new)
            .execute(&conn)
            .map(|res| {
                if res == 1 {
                    log::trace!("Row inserted (swap id: {})", md.swap_id);
                } else {
                    log::trace!("Row already exists (swap id: {})", md.swap_id);
                }
            })
            .map_err(|err| Error::Insert(err.to_string()))
    }

    fn all(&self) -> Result<Vec<Metadata>, Error> {
        // Imports aliases so we can refer to the table and table fields.
        use self::schema::metadatas::dsl::*;

        let conn = establish_connection(&self.db)?;

        metadatas
            .load::<Metadata>(&conn)
            .map_err(|err| Error::Load(err.to_string()))?
            .iter()
            .map(|m| Metadata::try_from(m.clone()))
            .collect()
    }
}

pub fn default_db_path() -> Option<PathBuf> {
    crate::data_dir().map(|dir| Path::join(&dir, "cnd.sqlite"))
}

fn create_database_if_needed(db: &Path) -> Result<(), Error> {
    if db.exists() {
        log::info!("Found Sqlite database: {}", db.display());
    } else {
        log::info!("Creating Sqlite database: {}", db.display());

        if let Some(parent) = db.parent() {
            std::fs::create_dir_all(parent).map_err(|err| Error::Path(err.to_string()))?;
        }

        File::create(db).map_err(|err| Error::Path(err.to_string()))?;
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
    type Error = Error;

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
