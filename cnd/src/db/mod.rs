mod models;
mod save_message;
mod schema;
embed_migrations!("./migrations");

pub use self::save_message::{SaveMessage, SaveRfc003Messages};
use crate::{
    db::models::{InsertableSwap, SqlText, Swap},
    swap_protocols::SwapId,
};
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

    /// Get swap with swap_id 'key' from the database.
    pub fn get(&self, key: SqlText<SwapId>) -> Result<Option<Swap>, Error> {
        use self::schema::swaps::dsl::*;

        let connection = self.connect()?;

        swaps
            .filter(swap_id.eq(key))
            .first(&connection)
            .optional()
            .map_err(Error::Diesel)
    }

    /// Inserts a swap into the database.  swap_id's are unique, attempting
    /// to insert a swap with a duplicate swap_id is an error.
    /// Returns 1 if a swap was inserted.
    pub fn insert(&self, swap: InsertableSwap) -> Result<usize, Error> {
        use self::schema::swaps::dsl::*;

        let connection = self.connect()?;

        diesel::insert_into(swaps)
            .values(&swap)
            .execute(&connection)
            .map_err(Error::Diesel)
    }

    /// Gets all the swaps from the database.
    pub fn all(&self) -> Result<Vec<Swap>, Error> {
        use self::schema::swaps::dsl::*;
        let connection = self.connect()?;

        swaps.load(&connection).map_err(Error::Diesel)
    }

    /// Deletes a swap with swap_id 'key' from the database.
    /// Returns 1 a swap was deleted, 0 otherwise.
    pub fn delete(&self, key: SqlText<SwapId>) -> Result<usize, Error> {
        use self::schema::swaps::dsl::*;

        let connection = self.connect()?;

        diesel::delete(swaps.filter(swap_id.eq(key)))
            .execute(&connection)
            .map_err(Error::Diesel)
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

    // Surely there is a more memory efficient way of doing this,
    // however, we only use this function for testing.
    #[allow(dead_code)]
    fn count(&self) -> Result<usize, Error> {
        use self::schema::swaps::dsl::*;

        let connection = self.connect()?;
        let records: Vec<Swap> = swaps.load(&connection).map_err(Error::Diesel)?;

        Ok(records.len())
    }
}

pub fn default_db_path() -> Option<PathBuf> {
    crate::data_dir().map(|dir| Path::join(&dir, "cnd.sqlite"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        db::models::{self, AssetKind, InsertableSwap, LedgerKind, Role},
        swap_protocols::SwapId,
    };
    use std::str::FromStr;

    fn instantiate_test_case_1() -> InsertableSwap {
        let swap_id = SwapId::from_str("aaaaaaaa-ecf2-4cc6-b35c-b4351ac28a34").unwrap();

        InsertableSwap::new(
            swap_id,
            LedgerKind::Bitcoin,
            LedgerKind::Ethereum,
            AssetKind::Bitcoin,
            AssetKind::Erc20,
            Role::Alice,
        )
    }

    fn instantiate_test_case_2() -> InsertableSwap {
        let swap_id = SwapId::from_str("bbbbbbbb-ecf2-4cc6-b35c-b4351ac28a34").unwrap();

        InsertableSwap::new(
            swap_id,
            LedgerKind::Ethereum,
            LedgerKind::Bitcoin,
            AssetKind::Erc20,
            AssetKind::Bitcoin,
            Role::Bob,
        )
    }

    impl PartialEq<models::Swap> for InsertableSwap {
        fn eq(&self, other: &models::Swap) -> bool {
            self.swap_id == other.swap_id
                && self.alpha_ledger == other.alpha_ledger
                && self.beta_ledger == other.beta_ledger
                && self.alpha_asset == other.alpha_asset
                && self.beta_asset == other.beta_asset
                && self.role == other.role
        }
    }

    // Argument types the opposite way around to eq() above.
    impl PartialEq<InsertableSwap> for models::Swap {
        fn eq(&self, other: &InsertableSwap) -> bool {
            other == self
        }
    }

    fn new_temp_db() -> Sqlite {
        let temp_file = tempfile::Builder::new()
            .suffix(".sqlite")
            .tempfile()
            .unwrap();
        let temp_file_path = temp_file.into_temp_path().to_path_buf();
        Sqlite::new(Some(temp_file_path)).expect("failed to create db")
    }

    #[test]
    fn can_create_a_new_temp_db() {
        let _db = new_temp_db();
    }

    #[test]
    fn can_count_empty_db() {
        let db = new_temp_db();
        assert_eq!(0, db.count().expect("db count error"));
    }

    #[test]
    fn insert_return_vaule_is_as_expected() {
        let db = new_temp_db();

        let swap = instantiate_test_case_1();
        let rows_inserted = db.insert(swap).expect("db insert error");
        assert_eq!(rows_inserted, 1);
    }

    #[test]
    fn count_works_afer_insert() {
        let db = new_temp_db();

        let swap = instantiate_test_case_1();
        let _rows_inserted = db.insert(swap).expect("db insert error");

        assert_eq!(1, db.count().expect("db count error"));
    }

    #[test]
    fn can_not_add_same_record_twice() {
        let db = new_temp_db();

        let swap = instantiate_test_case_1();
        let _rows_inserted = db.insert(swap).expect("db insert error");

        let swap_with_same_swap_id = instantiate_test_case_1();
        let res = db.insert(swap_with_same_swap_id);
        if res.is_ok() {
            panic!("insert duplicate swap_id should err");
        }

        assert_eq!(1, db.count().expect("db count error"));
    }

    #[test]
    fn can_add_multiple_different_swaps() {
        let db = new_temp_db();

        let s1 = instantiate_test_case_1();
        let s2 = instantiate_test_case_2();

        let rows_inserted = db.insert(s1).unwrap();
        assert_eq!(rows_inserted, 1);

        let rows_inserted = db.insert(s2).unwrap();
        assert_eq!(rows_inserted, 1);

        assert_eq!(2, db.count().expect("db count error"));
    }

    #[test]
    fn can_add_a_swap_and_read_it() {
        let db = new_temp_db();

        let swap = instantiate_test_case_1();
        let _rows_inserted = db.insert(swap).expect("db insert error");

        let res = db.get(swap.swap_id).expect("db get error");
        let got = res.unwrap();

        assert_eq!(got.swap_id, swap.swap_id);
    }

    #[test]
    fn can_add_a_swap_and_delete_it() {
        let db = new_temp_db();

        let swap = instantiate_test_case_1();
        let _rows_inserted = db.insert(swap).expect("db insert error");

        let rows_effected = db.delete(swap.swap_id).expect("db delete error");
        assert_eq!(rows_effected, 1);

        let swap = db.get(swap.swap_id).expect("db get error");
        if swap.is_some() {
            panic!("false positive");
        }

        assert_eq!(0, db.count().expect("db count error"));
    }

    #[test]
    fn can_delete_a_non_existant_swap() {
        let db = new_temp_db();
        let swap = instantiate_test_case_1();
        let rows_effected = db.delete(swap.swap_id).expect("db delete error");
        assert_eq!(rows_effected, 0);
    }

    #[test]
    fn can_get_all_the_swaps() {
        let db = new_temp_db();

        let s1 = instantiate_test_case_1();
        let s2 = instantiate_test_case_2();

        let _rows_inserted = db.insert(s1).expect("db insert 1 error");
        let _rows_inserted = db.insert(s2).expect("db insert 2 error");
        assert_eq!(2, db.count().expect("db count error"));

        let records = db.all().expect("db all error");
        assert_eq!(records.len(), 2);
    }

    #[test]
    fn swap_serializes_and_deserializes_correctly() {
        let db = new_temp_db();

        let swap = instantiate_test_case_1();
        let _rows_inserted = db.insert(swap).expect("db insert error");

        let res = db.get(swap.swap_id).expect("db get error");
        let got = res.unwrap();

        assert_eq!(got, swap);
    }
}
