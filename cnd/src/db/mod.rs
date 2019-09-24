mod models;
mod schema;
embed_migrations!("./migrations");

use crate::db::models::{InsertableSwap, Swap, SwapId};
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

        let connection = establish_connection(&db)?;
        embedded_migrations::run(&connection)?;

        Ok(Sqlite { db })
    }

    /// Get swap with swap_id 'key' from the database.
    pub fn get(&self, key: SwapId) -> Result<Option<Swap>, Error> {
        use self::schema::swaps::dsl::*;

        let connection = establish_connection(&self.db)?;

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

        let connection = establish_connection(&self.db)?;

        diesel::insert_into(swaps)
            .values(&swap)
            .execute(&connection)
            .map_err(Error::Diesel)
    }

    /// Gets all the swaps from the database.
    pub fn all(&self) -> Result<Vec<Swap>, Error> {
        use self::schema::swaps::dsl::*;
        let connection = establish_connection(&self.db)?;

        swaps.load(&connection).map_err(Error::Diesel)
    }

    /// Deletes a swap with swap_id 'key' from the database.
    /// Returns 1 a swap was deleted, 0 otherwise.
    pub fn delete(&self, key: SwapId) -> Result<usize, Error> {
        use self::schema::swaps::dsl::*;

        let connection = establish_connection(&self.db)?;

        diesel::delete(swaps.filter(swap_id.eq(key)))
            .execute(&connection)
            .map_err(Error::Diesel)
    }

    // Surely there is a more memory efficient way of doing this,
    // however, we only use this function for testing.
    #[allow(dead_code)]
    fn count(&self) -> Result<usize, Error> {
        use self::schema::swaps::dsl::*;

        let connection = establish_connection(&self.db)?;
        let records: Vec<Swap> = swaps.load(&connection).map_err(Error::Diesel)?;

        Ok(records.len())
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
    let connection = SqliteConnection::establish(&database_url).map_err(Error::Connection)?;

    Ok(connection)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::SwapId;
    use std::str::FromStr;

    fn new_temp_db() -> Sqlite {
        let temp_file = tempfile::Builder::new()
            .suffix(".sqlite")
            .tempfile()
            .unwrap();
        let temp_file_path = temp_file.into_temp_path().to_path_buf();
        Sqlite::new(Some(temp_file_path)).expect("failed to create database")
    }

    fn insertable_swap() -> InsertableSwap {
        let id = example_swap_id();
        create_insertable_swap(id)
    }

    fn another_insertable_swap() -> InsertableSwap {
        let id = another_example_swap_id();
        create_insertable_swap(id)
    }

    fn example_swap_id() -> SwapId {
        SwapId::from_str("aaaaaaaa-ecf2-4cc6-b35c-b4351ac28a34").unwrap()
    }

    fn another_example_swap_id() -> SwapId {
        SwapId::from_str("bbbbbbbb-ecf2-4cc6-b35c-b4351ac28a34").unwrap()
    }

    fn create_insertable_swap(swap_id: SwapId) -> InsertableSwap {
        InsertableSwap { swap_id }
    }

    #[test]
    fn can_create_a_new_temp_database() {
        let _db = new_temp_db();
    }

    #[test]
    fn can_count_empty_database() {
        let db = new_temp_db();
        assert_eq!(0, db.count().unwrap());
    }

    #[test]
    fn insert_return_vaule_is_as_expected() {
        let db = new_temp_db();

        let swap = insertable_swap();
        let rows_inserted = db.insert(swap).unwrap();
        assert_eq!(rows_inserted, 1);
    }

    #[test]
    fn count_works_afer_insert() {
        let db = new_temp_db();
        let swap = insertable_swap();

        let _rows_inserted = db.insert(swap).unwrap();
        assert_eq!(1, db.count().unwrap());
    }

    #[test]
    fn can_not_add_same_record_twice() {
        let db = new_temp_db();
        let swap = insertable_swap();

        let _res = db.insert(swap).unwrap();

        let swap_with_same_swap_id = insertable_swap();
        let res = db.insert(swap_with_same_swap_id);
        if res.is_ok() {
            panic!("insert duplicate swap_id should err");
        }

        assert_eq!(1, db.count().unwrap());
    }

    #[test]
    fn can_add_multiple_different_swaps() {
        let db = new_temp_db();

        let s1 = insertable_swap();
        let s2 = another_insertable_swap();

        let rows_inserted = db.insert(s1).unwrap();
        assert_eq!(rows_inserted, 1);

        let rows_inserted = db.insert(s2).unwrap();
        assert_eq!(rows_inserted, 1);

        assert_eq!(2, db.count().unwrap());
    }

    #[test]
    fn can_add_a_swap_and_read_it() {
        let db = new_temp_db();

        let swap_id = example_swap_id();
        let swap = create_insertable_swap(swap_id);

        let _ = db.insert(swap).unwrap();

        let swap = db.get(swap_id).expect("database error");
        match swap {
            Some(swap) => assert_eq!(swap.swap_id, swap_id),
            None => panic!("no record returned"),
        }
    }

    #[test]
    fn can_add_a_swap_and_delete_it() {
        let db = new_temp_db();

        let swap_id = example_swap_id();
        let swap = create_insertable_swap(swap_id);

        let _ = db.insert(swap).unwrap();

        let res = db.delete(swap_id.clone()).expect("database delete error");
        assert_eq!(res, 1);

        let swap = db.get(swap_id).expect("database get error");
        if swap.is_some() {
            panic!("false positive");
        }

        assert_eq!(0, db.count().unwrap());
    }

    #[test]
    fn can_delete_a_non_existant_swap() {
        let db = new_temp_db();

        let swap_id = example_swap_id();

        let res = db.delete(swap_id).expect("database delete error");
        assert_eq!(res, 0);
    }

    #[test]
    fn can_get_all_the_swaps() {
        let db = new_temp_db();

        let s1 = insertable_swap();
        let s2 = another_insertable_swap();

        let _ = db.insert(s1).unwrap();
        let _ = db.insert(s2).unwrap();

        let records = db.all().unwrap();
        assert_eq!(records.len(), 2);
    }
}
