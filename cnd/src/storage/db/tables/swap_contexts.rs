use crate::{
    local_swap_id::LocalSwapId,
    storage::{schema::swap_contexts, Text},
};
use comit::{LockProtocol, Role};

#[derive(Associations, Clone, Copy, Debug, Identifiable, Queryable, PartialEq)]
#[table_name = "swap_contexts"]
pub struct SwapContext {
    id: i32,
    pub local_swap_id: Text<LocalSwapId>,
    pub role: Text<Role>,
    pub alpha: Text<LockProtocol>,
    pub beta: Text<LockProtocol>,
}
