use crate::storage::{db::schema::secret_hashes, tables::Swap, Text};
use anyhow::Result;
use diesel::{prelude::*, sqlite::SqliteConnection};

#[derive(Associations, Clone, Copy, Debug, Identifiable, Queryable, PartialEq)]
#[belongs_to(Swap)]
#[table_name = "secret_hashes"]
pub struct SecretHash {
    id: i32,
    swap_id: i32,
    pub secret_hash: Text<comit::SecretHash>,
}

#[derive(Insertable, Debug, Clone, Copy)]
#[table_name = "secret_hashes"]
pub struct InsertableSecretHash {
    swap_id: i32,
    secret_hash: Text<comit::SecretHash>,
}

impl InsertableSecretHash {
    pub fn new(swap_fk: i32, secret_hash: comit::SecretHash) -> Self {
        Self {
            swap_id: swap_fk,
            secret_hash: Text(secret_hash),
        }
    }

    pub fn insert(self, conn: &SqliteConnection) -> Result<()> {
        diesel::insert_into(secret_hashes::table)
            .values(self)
            .execute(conn)?;

        Ok(())
    }
}
