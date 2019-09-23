use crate::db::schema::swaps;
use diesel::{Insertable, Queryable};

#[derive(Queryable, Debug, Clone)]
pub struct Swap {
    pub swap_id: String,
}

#[derive(Insertable, Debug)]
#[table_name = "swaps"]
pub struct InsertableSwap {
    pub swap_id: String,
}

impl From<Swap> for InsertableSwap {
    fn from(swap: Swap) -> Self {
        InsertableSwap {
            swap_id: swap.swap_id,
        }
    }
}
