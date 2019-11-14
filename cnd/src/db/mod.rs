mod custom_sql_types;
mod load_swaps;
mod new_types;
mod save_message;
mod schema;
#[cfg(test)]
mod serialization_format_stability_tests;
embed_migrations!("./migrations");

pub use self::save_message::{SaveMessage, SaveRfc003Messages};
use diesel::{self, prelude::*, sqlite::SqliteConnection};
use std::path::Path;

/// This module provides persistent storage by way of Sqlite.

#[derive(Debug, Clone)]
pub struct Sqlite {
    uri: String,
}

/// Defines the storage location of our SQLite database
#[derive(Debug)]
pub enum Location<'p> {
    OnDisk(&'p Path),
    #[cfg(test)]
    InMemory,
}

impl Sqlite {
    /// Return a handle that can be used to access the database.
    ///
    /// When this returns an Sqlite database exists at 'db', a
    /// successful connection to the database has been made, and
    /// the database migrations have been run.
    pub fn new(location: Location<'_>) -> anyhow::Result<Self> {
        let db = match location {
            Location::OnDisk(path) => {
                if path == Path::new(":memory:") {
                    anyhow::bail!("use Location::InMemory if you want an in-memory database!")
                }

                ensure_folder_tree_exists(path)?;

                Sqlite {
                    uri: format!("file:{}", path.display()),
                }
            }
            #[cfg(test)]
            Location::InMemory => Sqlite {
                uri: ":memory:".to_owned(),
            },
        };

        let connection = db.connect()?;
        embedded_migrations::run(&connection)?;

        Ok(db)
    }

    fn connect(&self) -> anyhow::Result<SqliteConnection> {
        Ok(SqliteConnection::establish(&self.uri)?)
    }
}

fn ensure_folder_tree_exists(path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    Ok(())
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

        let db = Sqlite::new(Location::OnDisk(&path));

        assert_that(&db).is_ok();
    }

    #[test]
    fn given_no_database_exists_calling_new_creates_it() {
        let path = temp_db();
        // validate assumptions: the db does not exist yet
        assert_that(&path.as_path()).does_not_exist();

        let db = Sqlite::new(Location::OnDisk(&path));

        assert_that(&db).is_ok();
        assert_that(&path.as_path()).exists();
    }

    #[test]
    fn given_db_in_non_existing_directory_tree_calling_new_creates_it() {
        let path = tempfile::tempdir()
            .unwrap()
            .into_path()
            .join("some_folder")
            .join("i_dont_exist")
            .join("database.sqlite")
            .to_path_buf();

        // validate assumptions:
        // 1. the db does not exist yet
        // 2. the parent folder does not exist yet
        assert_that(&path.as_path()).does_not_exist();
        assert_that(&path.as_path().parent())
            .is_some()
            .does_not_exist();

        let db = Sqlite::new(Location::OnDisk(&path));

        assert_that(&db).is_ok();
        assert_that(&path.as_path()).exists();
    }

    #[test]
    fn given_special_memory_path_does_not_create_a_file() {
        let result = Sqlite::new(Location::InMemory);

        assert_that(&result).is_ok();
        assert_that(&Path::new(":memory:")).does_not_exist();
    }

    #[test]
    fn given_memory_as_a_path_fails() {
        let result = Sqlite::new(Location::OnDisk(Path::new(":memory:")));

        assert_that(&result).is_err();
    }
}
