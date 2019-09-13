#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    SwapResponse(crate::client::RequestError),
    TimerError,
    IncorrectFunding,
    Internal(String),
}
