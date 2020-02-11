use crate::swap_protocols::han;
use crate::swap_protocols::herc20;

pub enum LedgerState<LA: han::Ledger, AA: han::Asset> {
    Han(han::LedgerState<L, A>),
    Herc20(herc20::LedgerState<L, A>),
}
