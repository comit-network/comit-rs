mod models;
mod schema;
embed_migrations!("./migrations");

use crate::{
    db::models::{InsertableMetadata, Metadata},
    swap_protocols::{
        metadata_store as ms,
        metadata_store::{AssetKind, LedgerKind, MetadataStore, Role},
        SwapId,
    },
};
use diesel::{self, prelude::*, sqlite::SqliteConnection};
use libp2p::PeerId;
use std::{
    convert::TryFrom,
    fs::File,
    io,
    path::{Path, PathBuf},
    str::FromStr,
};

/// This module provides an Sqlite backed persistent MetadataStore.

#[derive(Debug)]
pub enum Error {
    PathStr, // Cannot convert db path to a string.
    NoPath,  // Something went terminally wrong with the path string conversion.
    Connection(diesel::ConnectionError),
    Migrations(diesel_migrations::RunMigrationsError),
    Io(io::Error),
    Diesel(diesel::result::Error),
    Parse, // General parse error when reading from the database.
}

impl From<diesel::ConnectionError> for Error {
    fn from(err: diesel::ConnectionError) -> Error {
        Error::Connection(err)
    }
}

impl From<diesel_migrations::RunMigrationsError> for Error {
    fn from(err: diesel_migrations::RunMigrationsError) -> Error {
        Error::Migrations(err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<diesel::result::Error> for Error {
    fn from(err: diesel::result::Error) -> Error {
        Error::Diesel(err)
    }
}

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
        let db = db.or_else(default_db_path).ok_or(Error::NoPath)?;

        create_database_if_needed(&db)?;

        let conn = establish_connection(&db)?;

        embedded_migrations::run(&conn)?;

        Ok(SqliteMetadataStore { db })
    }
}

impl MetadataStore for SqliteMetadataStore {
    fn get(&self, key: SwapId) -> Result<Option<ms::Metadata>, ms::Error> {
        // Imports aliases so we can refer to the table and table fields.
        use self::schema::metadatas::dsl::*;

        let key = key.to_string();
        let conn = establish_connection(&self.db)?;

        metadatas
            .filter(swap_id.eq(key))
            .first(&conn)
            .optional()
            .map_err(|err| ms::Error::Sqlite(Error::Diesel(err)))?
            .map(|m: Metadata| ms::Metadata::try_from(m.clone()))
            .transpose()
    }

    fn insert(&self, metadata: ms::Metadata) -> Result<(), ms::Error> {
        let md = Metadata::new(metadata);
        let new = InsertableMetadata::new(&md);

        let conn = establish_connection(&self.db)?;
        diesel::insert_into(schema::metadatas::table)
            .values(new)
            .execute(&conn)
            .map_err(|err| ms::Error::Sqlite(Error::Diesel(err)))
            .map(|res| {
                if res == 1 {
                    log::trace!("Row inserted (swap id: {})", md.swap_id);
                } else {
                    log::trace!("Row already exists (swap id: {})", md.swap_id);
                }
            })
    }

    fn all(&self) -> Result<Vec<ms::Metadata>, ms::Error> {
        // Imports aliases so we can refer to the table and table fields.
        use self::schema::metadatas::dsl::*;

        let conn = establish_connection(&self.db)?;

        metadatas
            .load::<Metadata>(&conn)
            .map_err(|err| ms::Error::Sqlite(Error::Diesel(err)))?
            .iter()
            .map(|m| ms::Metadata::try_from(m.clone()))
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
            std::fs::create_dir_all(parent)?;
        }

        File::create(db)?;
    }
    Ok(())
}

fn establish_connection(db: &Path) -> Result<SqliteConnection, Error> {
    let database_url = db.to_str().ok_or(Error::PathStr)?;
    let conn = SqliteConnection::establish(&database_url)?;
    Ok(conn)
}

impl TryFrom<Metadata> for ms::Metadata {
    type Error = ms::Error;

    fn try_from(md: Metadata) -> Result<Self, Self::Error> {
        // These map_err calls can be removed once Metadata uses types and
        // implements the FromSql trait.
        let swap_id = SwapId::from_str(md.swap_id.as_str()).map_err(|_| Error::Parse)?;
        let alpha_ledger =
            LedgerKind::from_str(md.alpha_ledger.as_str()).map_err(|_| Error::Parse)?;
        let beta_ledger =
            LedgerKind::from_str(md.beta_ledger.as_str()).map_err(|_| Error::Parse)?;
        let alpha_asset = AssetKind::from_str(md.alpha_asset.as_str()).map_err(|_| Error::Parse)?;
        let beta_asset = AssetKind::from_str(md.beta_asset.as_str()).map_err(|_| Error::Parse)?;
        let role = Role::from_str(md.role.as_str()).map_err(|_| Error::Parse)?;
        let counterparty = PeerId::from_str(md.counterparty.as_str()).map_err(|_| Error::Parse)?;

        Ok(ms::Metadata {
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
