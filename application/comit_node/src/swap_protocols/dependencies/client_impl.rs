// FIXME: Figure out how to handle connection dependencies properly.
// I kept this separate here because Client really shouldn't be
// implemented on ProtocolDependencies. How do we make a request
// without passing in all the dependencies just in case we have to
// open up a new connection and decide how to respond to requests?
use crate::{
    bam_ext::{FromBamHeader, ToBamHeader},
    comit_client::{rfc003, Client, RequestError, SwapDeclineReason, SwapReject},
    node_id::NodeId,
    swap_protocols::{
        self, asset::Asset, dependencies::ProtocolDependencies, metadata_store::MetadataStore,
        rfc003::state_store::StateStore, swap_id::SwapId, SwapProtocols,
    },
};
use bam::{self, json, Status};
use futures::Future;
use tokio;

#[derive(Debug, Deserialize)]
pub struct Reason {
    pub value: SwapDeclineReason,
}

#[allow(type_alias_bounds)]
type SwapResponse<AL: swap_protocols::rfc003::Ledger, BL: swap_protocols::rfc003::Ledger> = Box<
    dyn Future<Item = Result<rfc003::AcceptResponseBody<AL, BL>, SwapReject>, Error = RequestError>
        + Send,
>;

impl<T: MetadataStore<SwapId>, S: StateStore<SwapId>> Client for ProtocolDependencies<T, S> {
    fn send_rfc003_swap_request<
        AL: swap_protocols::rfc003::Ledger,
        BL: swap_protocols::rfc003::Ledger,
        AA: Asset,
        BA: Asset,
    >(
        &self,
        node_id: NodeId,
        request: rfc003::Request<AL, BL, AA, BA>,
    ) -> SwapResponse<AL, BL> {
        Box::new(
            self.connection_pool
                .client_for(node_id, self.clone())
                .map_err(|e| {
                    RequestError::Connecting(e)
                })
                .and_then(move |client| {
                    let request = build_swap_request(request)
                        .expect("constructing a bam::json::OutoingRequest should never fail!");

                    debug!("Making swap request to {}: {:?}", node_id, request);

                    let response = client.lock().unwrap().send_request(request).then(
                        move |result| match result {
                            Ok(mut response) => match response.status() {
                                Status::OK(_) => {
                                    info!("{} accepted swap request: {:?}", node_id, response);
                                    match serde_json::from_value(response.body().clone()) {
                                        Ok(response) => Ok(Ok(response)),
                                        Err(_e) => Err(RequestError::InvalidResponse),
                                    }
                                }
                                Status::SE(20) => {
                                    info!("{} declined swap request: {:?}", node_id, response);
                                    Ok(Err({
                                        let reason = response
                                            .take_header("REASON")
                                            .map(SwapDeclineReason::from_bam_header)
                                            .map_or(Ok(None), |x| x.map(Some))
                                            .map_err(|e| {
                                                error!(
                                                    "Could not deserialize header in response {:?}: {}",
                                                    response, e,
                                                );
                                                RequestError::InvalidResponse
                                            })?;

                                        SwapReject::Declined { reason }
                                    }))
                                }
                                Status::SE(_) => {
                                    info!("{} rejected swap request: {:?}", node_id, response);
                                    Ok(Err(SwapReject::Rejected))
                                }
                                Status::RE(_) => {
                                    error!(
                                        "{} rejected swap request because of an internal error: {:?}",
                                        node_id, response
                                    );
                                    Err(RequestError::InternalError)
                                }
                            },
                            Err(e) => {
                                error!("Unable to request over connection {:?}:{:?}", node_id, e);
                                Err(RequestError::Connection)
                            }
                        },
                    );

                    Box::new(response)
                }),
        )
    }
}

fn build_swap_request<
    AL: swap_protocols::rfc003::Ledger,
    BL: swap_protocols::rfc003::Ledger,
    AA: Asset,
    BA: Asset,
>(
    request: rfc003::Request<AL, BL, AA, BA>,
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
        .with_header("swap_protocol", SwapProtocols::Rfc003.to_bam_header()?)
        .with_body(serde_json::to_value(rfc003::RequestBody::<AL, BL> {
            alpha_ledger_refund_identity,
            beta_ledger_redeem_identity,
            alpha_expiry,
            beta_expiry,
            secret_hash,
        })?))
}
