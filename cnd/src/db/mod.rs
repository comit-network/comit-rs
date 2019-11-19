mod custom_sql_types;
#[cfg(test)]
mod integration_tests;
mod load_swaps;
mod new_types;
mod save_message;
mod schema;
#[cfg(test)]
mod serialization_format_stability_tests;
mod swap;
mod swap_types;
#[macro_use]
pub mod with_swap_types;
embed_migrations!("./migrations");

pub use self::{
    save_message::{SaveMessage, SaveRfc003Messages},
    swap::*,
    swap_types::*,
};

use crate::{
    db::custom_sql_types::Text,
    swap_protocols::{Role, SwapId},
};
use diesel::{self, prelude::*, sqlite::SqliteConnection};
use std::{path::Path, thread, time};

/// This module provides persistent storage by way of Sqlite.

#[derive(Debug, Clone)]
pub struct Sqlite {
    uri: String,
}

impl Sqlite {
    /// Return a handle that can be used to access the database.
    ///
    /// When this returns an Sqlite database exists at 'db', a
    /// successful connection to the database has been made, and
    /// the database migrations have been run.
    pub fn new(path: &Path) -> anyhow::Result<Self> {
        ensure_folder_tree_exists(path)?;

        let db = Sqlite {
            uri: format!("file:{}", path.display()),
        };

        let connection = db.connect()?;
        embedded_migrations::run(&connection)?;

        Ok(db)
    }

    fn connect(&self) -> anyhow::Result<SqliteConnection> {
        let mut backoff = 10;

        loop {
            match SqliteConnection::establish(&self.uri) {
                Ok(connection) => return Ok(connection),
                Err(_) => {
                    thread::sleep(time::Duration::from_millis(backoff));
                    backoff *= 2;

                    if backoff > 1000 {
                        return Err(anyhow::Error::new(Error::ConnectionTimedOut));
                    }
                }
            }
        }
    }

    fn role(&self, key: &SwapId) -> anyhow::Result<Role> {
        use self::schema::rfc003_swaps as swaps;

        let connection = self.connect()?;
        let key = Text(key);

        let record: QueryableSwap = swaps::table
            .filter(swaps::swap_id.eq(key))
            .select((swaps::swap_id, swaps::role))
            .first(&connection)
            .optional()?
            .ok_or(Error::SwapNotFound)?;

        Ok(*record.role)
    }
}

fn ensure_folder_tree_exists(path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    Ok(())
}

#[derive(Queryable, Debug, Clone, PartialEq)]
struct QueryableSwap {
    pub swap_id: Text<SwapId>,
    pub role: Text<Role>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("swap not found")]
    SwapNotFound,
    #[error("connection timed out")]
    ConnectionTimedOut,
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
