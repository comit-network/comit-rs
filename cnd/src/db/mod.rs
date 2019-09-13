mod models;
mod schema;

use crate::db::models::Metadata;

use crate::{
    db::models::InsertableMetadata,
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
    io::{stdout, Write},
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
            .or_else(default_db_path)
            .ok_or_else(|| Error::Path("Failed to get database path".to_string()))?;

        let migrations = default_migrations_dir()
            .ok_or_else(|| Error::Path("Failed to get migrations directory".to_string()))?;

        create_database_if_needed(&db)?;
        create_migrations_if_needed(&migrations)?;

        let conn = establish_connection(&db)?;
        run_migrations(conn, &migrations)?;

        Ok(SqliteMetadataStore { db })
    }
}

impl MetadataStore for SqliteMetadataStore {
    fn get(&self, key: SwapId) -> Result<Option<metadata_store::Metadata>, Error> {
        // Imports aliases so we can refer to the table and table fields.
        use self::schema::metadatas::dsl::*;

        let key = key.to_string();
        let conn = establish_connection(&self.db)?;

        metadatas
            .filter(swap_id.eq(key))
            .first(&conn)
            .optional()
            .map_err(|err| Error::Load(err.to_string()))?
            .map(|m: Metadata| metadata_store::Metadata::try_from(m.clone()))
            .transpose()
    }

    fn insert(&self, metadata: metadata_store::Metadata) -> Result<(), metadata_store::Error> {
        let md = Metadata::new(metadata);
        let new = InsertableMetadata::new(&md);

        let conn = establish_connection(&self.db)?;
        diesel::insert_into(schema::metadatas::table)
            .values(new)
            .execute(&conn)
            .map(|res| {
                // MetadataStore trait does not return any indication whether the
                // insert was succesful or not so just log it and drop the return value.
                // If/when InMemoryMetadataStore goes away we can probably change the trait.
                if res == 1 {
                    log::trace!("Row inserted (swap id: {})", md.swap_id);
                }
            })
            .map_err(|err| Error::Insert(err.to_string()))
    }

    fn all(&self) -> Result<Vec<metadata_store::Metadata>, Error> {
        // Imports aliases so we can refer to the table and table fields.
        use self::schema::metadatas::dsl::*;

        let conn = establish_connection(&self.db)?;

        metadatas
            .load::<Metadata>(&conn)
            .map_err(|err| Error::Load(err.to_string()))?
            .iter()
            .map(|m| metadata_store::Metadata::try_from(m.clone()))
            .collect()
    }
}

pub fn default_db_path() -> Option<PathBuf> {
    crate::data_dir().map(|dir| Path::join(&dir, "cnd.sqlite"))
}

pub fn default_migrations_dir() -> Option<PathBuf> {
    crate::data_dir().map(|dir| Path::join(&dir, "migrations"))
}

fn create_database_if_needed(db: &Path) -> Result<(), Error> {
    if db.exists() {
        log::info!("Found Sqlite database: {}", db.display());
    } else {
        log::info!("Creating Sqlite database: {}", db.display());
        let _file = File::create(db).map_err(|err| Error::Path(err.to_string()))?;
    }
    Ok(())
}

// Simple creation of initial migrations, logic could be improved.
fn create_migrations_if_needed(migrations: &Path) -> Result<(), Error> {
    if !migrations.exists() {
        std::fs::create_dir(migrations).map_err(|err| Error::Path(err.to_string()))?;
    }

    let initial = migrations.join("initial");
    if !initial.exists() {
        std::fs::create_dir(initial.clone()).map_err(|err| Error::Path(err.to_string()))?;
    }

    let down = initial.join("down.sql");
    if !down.exists() {
        let content = "DROP TABLE metadatas\n";
        write_to_file(&down, content)?;
    }

    let up = initial.join("up.sql");
    if !up.exists() {
        let content = "CREATE TABLE metadatas (
swap_id VARCHAR(255) NOT NULL PRIMARY KEY,
alpha_ledger VARCHAR(255) NOT NULL,
beta_ledger VARCHAR(255) NOT NULL,
alpha_asset VARCHAR(255) NOT NULL,
beta_asset VARCHAR(255) NOT NULL,
role VARCHAR(255) NOT NULL,
counterparty VARCHAR(255) NOT NULL
)
";
        write_to_file(&down, content)?;
    }

    Ok(())
}

fn establish_connection(db: &Path) -> Result<SqliteConnection, Error> {
    let database_url = db
        .to_str()
        .ok_or_else(|| Error::Path(format!("Database path invalid: {}", db.to_string_lossy())))?;

    SqliteConnection::establish(&database_url).map_err(|err| Error::Connect(err.to_string()))
}

// NOTE: This runs ALL migrations in migrations directory.  Once we add a second
// migration we should warn the user so we do not clobber their database.
fn run_migrations(conn: SqliteConnection, migrations: &Path) -> Result<(), Error> {
    migrations_internals::run_pending_migrations_in_directory(&conn, migrations, &mut stdout())
        .map_err(|err| Error::Init(err.to_string()))?;

    Ok(())
}

fn write_to_file(dst: &Path, content: &str) -> Result<(), Error> {
    let mut ofile = File::create(dst).map_err(|err| Error::Init(err.to_string()))?;

    ofile
        .write_all(content.as_bytes())
        .map_err(|err| Error::Init(err.to_string()))
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
