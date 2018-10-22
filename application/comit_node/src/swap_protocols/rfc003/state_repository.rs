use swap_protocols::rfc003::{ledger::Ledger, state_machine::SwapStates};

trait StateRepo<K, SL: Ledger, TL: Ledger, SA, TA> {
    fn set(id: K, state: SwapStates<SL, TL, SA, TA>);
    fn get(id: K) -> SwapStates<SL, TL, SA, TA>;
}
