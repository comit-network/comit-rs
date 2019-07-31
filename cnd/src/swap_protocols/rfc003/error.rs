use crate::{btsieve, comit_client};

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    SwapResponse(comit_client::RequestError),
    Btsieve(btsieve::Error),
    TimerError,
    InvalidFunding,
    Internal(String),
}
