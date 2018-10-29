use comit_client::SwapResponseError;
use failure;
use ledger_query_service;

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    SwapResponse(SwapResponseError),
    LedgerQueryService(String),
    TimerError,
    InsufficientFunding,
}

impl From<ledger_query_service::Error> for Error {
    fn from(e: ledger_query_service::Error) -> Self {
        Error::LedgerQueryService(failure::Error::from(e).to_string())
    }
}
