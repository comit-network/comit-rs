mod models;
mod save_message;
mod schema;
embed_migrations!("./migrations");

pub use self::save_message::{SaveMessage, SaveRfc003Messages};
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

#[derive(Debug, Clone)]
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
        let file = db.or_else(default_db_path).ok_or(Error::NoPath)?;
        let db = Sqlite { db: file };

        db.create_database_if_needed()?;
        let connection = db.connect()?;
        embedded_migrations::run(&connection)?;

        Ok(db)
    }

    fn create_database_if_needed(&self) -> Result<(), Error> {
        let db = &self.db;

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

    fn connect(&self) -> Result<SqliteConnection, Error> {
        let database_url = self.db.to_str().ok_or(Error::PathConversion)?;
        let connection = SqliteConnection::establish(&database_url)?;
        Ok(connection)
    }
}

pub fn default_db_path() -> Option<PathBuf> {
    crate::data_dir().map(|dir| Path::join(&dir, "cnd.sqlite"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectral::prelude::*;

    #[test]
    fn can_create_a_new_temp_db() {
        let temp_file = tempfile::Builder::new()
            .suffix(".sqlite")
            .tempfile()
            .unwrap();
        let temp_file_path = temp_file.into_temp_path().to_path_buf();

        let db = Sqlite::new(Some(temp_file_path));

        assert_that(&db).is_ok();
    }
}
