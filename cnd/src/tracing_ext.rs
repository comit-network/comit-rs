use crate::swap_protocols::LocalSwapId;
use comit::{Protocol, Role, Side};
use tracing_futures::{Instrument, Instrumented};

/// Extension trait for easily applying a consistent span across all protocol
/// instantiations.
pub trait InstrumentProtocol: Sized {
    fn instrument_protocol(
        self,
        id: LocalSwapId,
        role: Role,
        side: Side,
        protocol: Protocol,
    ) -> Instrumented<Self> {
        self.instrument(tracing::error_span!("", swap_id = %id, role = %role, side = %side, protocol = %protocol))
    }
}

impl<T> InstrumentProtocol for T {}
