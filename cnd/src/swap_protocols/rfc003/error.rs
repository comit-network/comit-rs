use crate::comit_client;

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    SwapResponse(comit_client::RequestError),
    Btsieve,
    TimerError,
    IncorrectFunding,
    Internal(String),
}
