use crate::db::schema::swaps;
use diesel::{Insertable, Queryable};

#[derive(Queryable, Debug, Clone, PartialEq)]
pub struct Swap {
    id: i32,
    pub swap_id: String,
}

#[derive(Insertable, Debug)]
#[table_name = "swaps"]
pub struct InsertableSwap {
    pub swap_id: String,
}

impl InsertableSwap {
    pub fn new(swap_id: &str) -> Self {
        InsertableSwap {
            swap_id: swap_id.to_string(),
        }
    }
}
