use crate::{local_swap_id::LocalSwapId, storage::Text};
use comit::{LockProtocol, Role};

#[derive(Associations, Clone, Copy, Debug, Queryable, PartialEq)]
#[table_name = "swap_contexts"]
pub struct SwapContext {
    #[diesel(deserialize_as = "Text<LocalSwapId>")]
    pub id: LocalSwapId,
    #[diesel(deserialize_as = "Text<Role>")]
    pub role: Role,
    #[diesel(deserialize_as = "Text<LockProtocol>")]
    pub alpha: LockProtocol,
    #[diesel(deserialize_as = "Text<LockProtocol>")]
    pub beta: LockProtocol,
}
