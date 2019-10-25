use crate::swap_protocols::{
    asset::Asset,
    metadata_store,
    rfc003::{self, Ledger},
};
use http_api_problem::HttpApiProblem;
use libp2p::PeerId;

#[derive(Debug)]
pub enum Error {
    Metadata(metadata_store::Error),
}

impl From<Error> for HttpApiProblem {
    fn from(e: Error) -> Self {
        use self::Error::*;
        match e {
            Metadata(e) => e.into(),
        }
    }
}

pub trait InsertState: Send + Sync + 'static {
    #[allow(clippy::type_complexity)]
    fn insert_state_into_stores<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>(
        &self,
        counterparty: PeerId,
        swap_request: rfc003::messages::Request<AL, BL, AA, BA>,
    ) -> Result<(), Error>;
}
