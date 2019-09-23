mod models;
mod schema;
embed_migrations!("./migrations");

use crate::db::models::Swap;
use diesel::{self, prelude::*, sqlite::SqliteConnection};
use std::{
    fs::File,
    io,
    path::{Path, PathBuf},
};

/// This module provides persistent storage by way of Sqlite.

#[derive(Debug)]
pub enum Error {
    PathConversion, // Cannot convert db path to a string.
    NoPath,         // Failed to generate a valid path for the database file.
    Connection(diesel::ConnectionError),
    Migrations(diesel_migrations::RunMigrationsError),
    Io(io::Error),
    Diesel(diesel::result::Error),
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
pub struct Sqlite {
    db: PathBuf,
}

impl Sqlite {
    /// Return a handle that can be used to access the database.
    ///
    /// When this returns an Sqlite database exists at 'db', a
    /// successful connection to the database has been made, and
    /// the database migrations have been run.
    pub fn new(db: Option<PathBuf>) -> Result<Sqlite, Error> {
        let db = db.or_else(default_db_path).ok_or(Error::NoPath)?;

        create_database_if_needed(&db)?;

        let conn = establish_connection(&db)?;
        embedded_migrations::run(&conn)?;

        Ok(Sqlite { db })
    }
}

pub trait Database: Send + Sync + 'static {
    fn get(&self, swap_id: String) -> Result<Option<Swap>, Error>;
    fn insert(&self, swap: Swap) -> Result<(), Error>;
    fn all(&self) -> Result<Vec<Swap>, Error>;
    fn delete(&self, swap_id: String) -> Result<(), Error>;
}

impl Database for Sqlite {
    fn get(&self, _swap_id: String) -> Result<Option<Swap>, Error> {
        unimplemented!()
    }

    fn insert(&self, _swap: Swap) -> Result<(), Error> {
        unimplemented!()
    }

    fn all(&self) -> Result<Vec<Swap>, Error> {
        unimplemented!()
    }

    fn delete(&self, _swap_id: String) -> Result<(), Error> {
        unimplemented!()
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
    let database_url = db.to_str().ok_or(Error::PathConversion)?;
    let conn = SqliteConnection::establish(&database_url)?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_create_a_new_database() {
        let temp_file = tempfile::Builder::new()
            .suffix(".sqlite")
            .tempfile()
            .unwrap();
        let temp_file_path = temp_file.into_temp_path().to_path_buf();
        let _db = Sqlite::new(Some(temp_file_path)).expect("failed to create database");
    }
}
