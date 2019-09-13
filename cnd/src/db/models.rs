use crate::{db::schema::metadatas, swap_protocols::metadata_store};
use diesel::{Insertable, Queryable};

#[derive(Queryable, Debug, Clone)]
pub struct Metadata {
    pub swap_id: String,
    pub alpha_ledger: String,
    pub beta_ledger: String,
    pub alpha_asset: String,
    pub beta_asset: String,
    pub role: String,
    pub counterparty: String,
}

impl Metadata {
    pub fn new(md: metadata_store::Metadata) -> Self {
        Metadata {
            swap_id: md.swap_id.to_string(),
            alpha_ledger: md.alpha_ledger.to_string(),
            beta_ledger: md.beta_ledger.to_string(),
            alpha_asset: md.alpha_asset.to_string(),
            beta_asset: md.beta_asset.to_string(),
            role: md.role.to_string(),
            counterparty: md.counterparty.to_string(),
        }
    }
}

// Diesel docs say to use this second structure for database inserts.
#[derive(Insertable, Debug)]
#[table_name = "metadatas"]
pub struct InsertableMetadata<'a> {
    pub swap_id: &'a str,
    pub alpha_ledger: &'a str,
    pub beta_ledger: &'a str,
    pub alpha_asset: &'a str,
    pub beta_asset: &'a str,
    pub role: &'a str,
    pub counterparty: &'a str,
}

impl<'a> InsertableMetadata<'a> {
    pub fn new(md: &'a Metadata) -> InsertableMetadata<'a> {
        InsertableMetadata {
            swap_id: &md.swap_id[..],
            alpha_ledger: &md.alpha_ledger[..],
            beta_ledger: &md.beta_ledger[..],
            alpha_asset: &md.alpha_asset[..],
            beta_asset: &md.beta_asset[..],
            role: &md.role[..],
            counterparty: &md.counterparty[..],
        }
    }
}
