use crate::{command::MigrateDb, database, database::Database};
use anyhow::{bail, Result};
use std::path::PathBuf;
use time::OffsetDateTime;

pub async fn migrate_db(action: MigrateDb, db_dir: &PathBuf) -> Result<()> {
    match action {
        MigrateDb::Status => {
            match status(db_dir).await? {
                MigrationStatus::NotNeeded => {
                    println!("Database migration is not needed.");
                }
                MigrationStatus::NeededSledFormat => println!(
                    "Database migration is needed due to sled format upgrade. Backup your database located at '{:?}' first.",
                    db_dir
                ),
                MigrationStatus::NeededSerializationFormat => println!(
                    "Database migration is needed due to nectar format upgrade. Backup your database located at '{:?}' first.",
                    db_dir
                ),
            }
            Ok(())
        }
        MigrateDb::Run => run(db_dir).await,
    }
}

async fn status(db_dir: &PathBuf) -> Result<MigrationStatus> {
    let db = match Database::new(db_dir.as_path()) {
        Err(database::Error::Sled(sled::Error::Unsupported(_))) => {
            println!("Database needs migration due to an upgrade of sled data type");
            return Ok(MigrationStatus::NeededSledFormat);
        }
        Err(err) => bail!("Issue opening the db: {:#?}", err),
        Ok(db) => db,
    };

    if db.are_any_swap_stored_in_old_format().await? {
        Ok(MigrationStatus::NeededSerializationFormat)
    } else {
        Ok(MigrationStatus::NotNeeded)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MigrationStatus {
    NotNeeded,
    NeededSledFormat,
    NeededSerializationFormat,
}

async fn run(db_dir: &PathBuf) -> Result<()> {
    match status(db_dir).await? {
        MigrationStatus::NotNeeded => {
            bail!("Database migration is not necessary");
        }

        MigrationStatus::NeededSledFormat => {
            let now = OffsetDateTime::now_local().lazy_format("%Y-%0m-%d_%0H%0M%0S.%N");
            let backup_dir_name = format!("database_bk_{}", now);
            let backup_dir = db_dir
                .parent()
                .ok_or_else(|| {
                    anyhow::anyhow!("failed to get the parent folder of the database directory")
                })?
                .join(backup_dir_name);

            std::fs::rename(db_dir, backup_dir.clone())?;

            let old_db = sled::open(backup_dir.as_path())?;
            let new_db = sled::open(db_dir.as_path())?;

            let export = old_db.export();

            new_db.import(export);

            Ok(())
        }
        MigrationStatus::NeededSerializationFormat => {
            let db = Database::new(db_dir.as_path())?;
            db.reserialize().await
        }
    }
}
