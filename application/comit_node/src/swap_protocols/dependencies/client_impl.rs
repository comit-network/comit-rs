// FIXME: Figure out how to handle connection dependencies properly.
// I kept this separate here because Client really shouldn't be
// implemented on ProtocolDependencies. How do we make a request
// without passing in all the dependencies just in case we have to
// open up a new connection and decide how to respond to requests?
use crate::{
    bam_ext::{FromBamHeader, ToBamHeader},
    comit_client::{Client, RequestError, SwapDeclineReason, SwapReject},
    network::{Behaviour, DialInformation},
    swap_protocols::{
        self,
        asset::Asset,
        rfc003::{self, bob::BobSpawner},
        SwapProtocol,
    },
};
use bam::{self, json, Status};
use futures::Future;
use libp2p::{Swarm, Transport};
use serde::Deserialize;
use std::sync::Mutex;
use tokio::{io::AsyncRead, prelude::AsyncWrite};

#[derive(Debug, Deserialize)]
pub struct Reason {
    pub value: SwapDeclineReason,
}

#[allow(type_alias_bounds)]
type SwapResponse<AL: swap_protocols::rfc003::Ledger, BL: swap_protocols::rfc003::Ledger> = Box<
    dyn Future<
            Item = Result<rfc003::messages::AcceptResponseBody<AL, BL>, SwapReject>,
            Error = RequestError,
        > + Send,
>;

impl<
        B: BobSpawner,
        TTransport: Transport + Send + 'static,
        TSubstream: AsyncRead + AsyncWrite + Send + 'static,
    > Client for Mutex<Swarm<TTransport, Behaviour<TSubstream, B>>>
where
    <TTransport as Transport>::Listener: Send,
    <TTransport as Transport>::Error: Send,
{
    fn send_rfc003_swap_request<
        AL: swap_protocols::rfc003::Ledger,
        BL: swap_protocols::rfc003::Ledger,
        AA: Asset,
        BA: Asset,
    >(
        &self,
        dial_information: DialInformation,
        request: rfc003::messages::Request<AL, BL, AA, BA>,
    ) -> SwapResponse<AL, BL> {
        let request = build_swap_request(request)
            .expect("constructing a bam::json::OutoingRequest should never fail!");

        let response = {
            let mut swarm = self.lock().unwrap();
            log::debug!(
                "Making swap request to {}: {:?}",
                dial_information.clone(),
                request
            );

            swarm.send_request(dial_information.clone(), request)
        };

        let response = response.then(move |result| match result {
            Ok(mut response) => match response.status() {
                Status::OK(_) => {
                    log::info!(
                        "{} accepted swap request: {:?}",
                        dial_information.clone(),
                        response
                    );
                    match serde_json::from_value(response.body().clone()) {
                        Ok(response) => Ok(Ok(response)),
                        Err(_e) => Err(RequestError::InvalidResponse),
                    }
                }
                Status::SE(20) => {
                    log::info!(
                        "{} declined swap request: {:?}",
                        dial_information.clone(),
                        response
                    );
                    Ok(Err({
                        let reason = response
                            .take_header("REASON")
                            .map(SwapDeclineReason::from_bam_header)
                            .map_or(Ok(None), |x| x.map(Some))
                            .map_err(|e| {
                                log::error!(
                                    "Could not deserialize header in response {:?}: {}",
                                    response,
                                    e,
                                );
                                RequestError::InvalidResponse
                            })?;

                        SwapReject::Declined { reason }
                    }))
                }
                Status::SE(_) => {
                    log::info!(
                        "{} rejected swap request: {:?}",
                        dial_information.clone(),
                        response
                    );
                    Ok(Err(SwapReject::Rejected))
                }
                Status::RE(_) => {
                    log::error!(
                        "{} rejected swap request because of an internal error: {:?}",
                        dial_information.clone(),
                        response
                    );
                    Err(RequestError::InternalError)
                }
            },
            Err(e) => {
                log::error!(
                    "Unable to request over connection {:?}:{:?}",
                    dial_information.clone(),
                    e
                );
                Err(RequestError::Connection)
            }
        });

        Box::new(response)
    }
}

fn build_swap_request<
    AL: swap_protocols::rfc003::Ledger,
    BL: swap_protocols::rfc003::Ledger,
    AA: Asset,
    BA: Asset,
>(
    request: rfc003::messages::Request<AL, BL, AA, BA>,
) -> Result<json::OutgoingRequest, serde_json::Error> {
    let alpha_ledger_refund_identity = request.alpha_ledger_refund_identity;
    let beta_ledger_redeem_identity = request.beta_ledger_redeem_identity;
    let alpha_expiry = request.alpha_expiry;
    let beta_expiry = request.beta_expiry;
    let secret_hash = request.secret_hash;

    Ok(json::OutgoingRequest::new("SWAP")
        .with_header("alpha_ledger", request.alpha_ledger.into().to_bam_header()?)
        .with_header("beta_ledger", request.beta_ledger.into().to_bam_header()?)
        .with_header("alpha_asset", request.alpha_asset.into().to_bam_header()?)
        .with_header("beta_asset", request.beta_asset.into().to_bam_header()?)
        .with_header("protocol", SwapProtocol::Rfc003.to_bam_header()?)
        .with_body(serde_json::to_value(rfc003::messages::RequestBody::<
            AL,
            BL,
        > {
            alpha_ledger_refund_identity,
            beta_ledger_redeem_identity,
            alpha_expiry,
            beta_expiry,
            secret_hash,
        })?))
}
