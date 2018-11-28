use comit_client::SwapResponseError;
use ledger_query_service;

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    SwapResponse(SwapResponseError),
    LedgerQueryService(ledger_query_service::Error),
    TimerError,
    InsufficientFunding,
    HtlcDeployment,
    Internal(String),
}
