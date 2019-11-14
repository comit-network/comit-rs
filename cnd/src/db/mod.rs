mod custom_sql_types;
mod load_swaps;
mod new_types;
mod save_message;
mod schema;
#[cfg(test)]
mod serialization_format_stability_tests;
embed_migrations!("./migrations");

pub use self::save_message::{SaveMessage, SaveRfc003Messages};
use anyhow::Context;
use diesel::{self, prelude::*, sqlite::SqliteConnection};
use std::{
    fs::File,
    path::{Path, PathBuf},
};

/// This module provides persistent storage by way of Sqlite.

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
    pub fn new(db: Option<PathBuf>) -> anyhow::Result<Sqlite> {
        let file = db
            .or_else(default_db_path)
            .context("failed to determine default path for database ")?;
        let db = Sqlite { db: file };

        db.create_database_if_needed()?;
        let connection = db.connect()?;
        embedded_migrations::run(&connection)?;

        Ok(db)
    }

    fn create_database_if_needed(&self) -> anyhow::Result<()> {
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

    fn connect(&self) -> anyhow::Result<SqliteConnection> {
        let db = &self.db;
        let database_url = db
            .to_str()
            .with_context(|| format!("{} is not a valid path", db.display()))?;
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
