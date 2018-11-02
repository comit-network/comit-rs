use comit_client::SwapResponseError;

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    SwapResponse(SwapResponseError),
    LedgerQueryService,
    TimerError,
    InsufficientFunding,
    Internal(String),
}
