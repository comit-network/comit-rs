use crate::{comit_client, ledger_query_service};

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    SwapResponse(comit_client::RequestError),
    LedgerQueryService(ledger_query_service::Error),
    TimerError,
    InsufficientFunding,
    Internal(String),
}
