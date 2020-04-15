use crate::swap_protocols::rfc003::SecretHash;
use crate::swap_protocols::NodeLocalSwapId;
use crate::{network::oneshot_protocol, swap_protocols::SwapId};
use serde::{Deserialize, Serialize};

// TODO: This might have to be changed to be a BehaviourOutEvent rather than using the oneshot

/// The message for an internal event emit after finalized to trigger the execution
#[derive(Clone, Copy, Deserialize, Debug, Serialize)]
pub struct Message {
    pub local_swap_id: NodeLocalSwapId,
    pub swap_id: SwapId,
    pub swap_params: CreatteSwapParams,
    pub secret_hash: SecretHash,
    pub invoice_state: InvoiceState,
}

impl Message {
    pub fn new(
        local_swap_id: NodeLocalSwapId,
        swap_id: SwapId,
        swap_params: CreatteSwapParams,
        secret_hash: SecretHash,
        invoice_state: InvoiceState,
    ) -> Self {
        Self {
            local_swap_id,
            swap_id,
            swap_params,
            secret_hash,
            invoice_state,
        }
    }
}

impl oneshot_protocol::Message for Message {
    const INFO: &'static str = "/comit/swap/finalized/1.0.0";
}
