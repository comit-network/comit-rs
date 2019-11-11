mod custom_sql_types;
mod new_types;
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

/// A suite of tests that ensures the serialization format of the types we use
/// to interact with the database. Changing the format needs to be a conscious
/// activity that involves migration scripts to migrate old data. These tests
/// make sure we don't change the format accidentally!
#[cfg(test)]
mod database_serialization_format_stability_tests {
    use crate::{
        db::new_types::{DecimalU256, EthereumAddress, Satoshis},
        swap_protocols::{rfc003::SecretHash, HashFunction, SwapId},
    };
    use std::{fmt, str::FromStr};

    #[test]
    fn swap_id() {
        test::<SwapId>("7f3a105d-ecf2-4cc6-b35c-b4351ac28a34")
    }

    #[test]
    fn bitcoin_network() {
        test::<bitcoin::Network>("bitcoin");
        test::<bitcoin::Network>("testnet");
        test::<bitcoin::Network>("regtest");
    }

    #[test]
    fn decimal_u256() {
        test::<DecimalU256>("1000000000000000");
    }

    #[test]
    fn bitcoin_amount() {
        test::<Satoshis>("100000000000");
    }

    #[test]
    fn hash_function() {
        test::<HashFunction>("SHA-256");
        assert_num_variants::<HashFunction>(1)
    }

    #[test]
    fn bitcoin_public_key() {
        test::<bitcoin::PublicKey>(
            "0216867374f539badfd90d7b2269008d893ae7bd4f9ee7c695c967d01d6953c401",
        );
    }

    #[test]
    fn ethereum_address() {
        test::<EthereumAddress>("68917b35bacf71dbadf37628b3b7f290f6d88877");
    }

    #[test]
    fn secrethash() {
        test::<SecretHash>("68917b35bacf71dbadf37628b3b7f290f6d88877d7b2269008d893ae7bd4f9ee");
    }

    fn test<T: fmt::Display + FromStr>(stored_value: &str)
    where
        <T as FromStr>::Err: fmt::Debug,
    {
        // First, we verify that we can create T from the given value.
        let read = T::from_str(stored_value).unwrap();

        // Next we convert it to a string again.
        let written = read.to_string();

        // Then if we end up with the same value, our serialization is stable.
        assert_eq!(written, stored_value)
    }

    fn assert_num_variants<E>(expected_number_of_variants: usize)
    where
        E: strum::IntoEnumIterator,
        <E as strum::IntoEnumIterator>::Iterator: Iterator,
    {
        let number_of_variants = E::iter().count();

        assert_eq!(
            number_of_variants,
            expected_number_of_variants,
            "the number of variants for this enum seem to have changed, please add a serialization format test for the new variant and update the expected variant count"
        )
    }
}
