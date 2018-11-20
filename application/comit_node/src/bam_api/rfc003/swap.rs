use bam::{
    config::Config,
    json::{Request, Response},
    Status,
};
use bam_api::header::FromBamHeader;
use comit_client::{self, rfc003::RequestBody};
use futures::{
    future::Future,
    sync::{mpsc, oneshot},
};
use swap_protocols::{
    asset::Asset,
    rfc003::{self, Ledger},
    SwapProtocols,
};
use swaps::common::SwapId;

pub fn swap_config(
    sender: mpsc::UnboundedSender<(
        SwapId,
        rfc003::bob::SwapRequestKind,
        oneshot::Sender<Result<rfc003::bob::SwapResponseKind, failure::Error>>,
    )>,
) -> Config<Request, Response> {
    Config::default().on_request(
        "SWAP",
        &[
            "beta_ledger",
            "alpha_ledger",
            "beta_asset",
            "alpha_asset",
            "swap_protocol",
        ],
        move |request: Request| {
            let swap_protocol = header!(request.get_header("swap_protocol"));

            match SwapProtocols::from_bam_header(swap_protocol).unwrap() {
                SwapProtocols::Rfc003 => {
                    let swap_id = SwapId::default();
                    let (response_sender, response_receiver) = oneshot::channel();

                    if let Ok(swap_request) = decode_request(&request) {
                        let request_kind =
                            rfc003::bob::SwapRequestKind::BitcoinEthereumBitcoinQuantityEthereumQuantity(
                                swap_request,
                            );
                        sender.unbounded_send((swap_id, request_kind, response_sender)).unwrap();
                    }

                    Box::new(response_receiver.then(move |result| {
                        match result {
                            Ok(Ok(rfc003::bob::SwapResponseKind::BitcoinEthereum(response))) => Ok(response.into()),
                            Ok(Err(e)) => {
                                error!("Error while processing swap request {}: {:?}", swap_id, e);
                                Ok(Response::new(Status::SE(0)))
                            },
                            Err(_) => {
                                warn!("Failed to receive from oneshot channel for swap {}", swap_id);
                                Ok(Response::new(Status::SE(0)))
                            }
                        }
                    }))
                }
            }
        },
    )
}

impl<AL: Ledger, BL: Ledger> From<rfc003::bob::SwapResponse<AL, BL>> for Response {
    fn from(response: rfc003::bob::SwapResponse<AL, BL>) -> Self {
        match response {
            rfc003::bob::SwapResponse::Accept {
                beta_ledger_refund_identity,
                alpha_ledger_success_identity,
                beta_ledger_lock_duration,
            } => {
                Response::new(Status::OK(20)).with_body(comit_client::rfc003::AcceptResponseBody::<
                    AL,
                    BL,
                > {
                    beta_ledger_refund_identity,
                    alpha_ledger_success_identity,
                    beta_ledger_lock_duration,
                })
            }
            rfc003::bob::SwapResponse::Decline => Response::new(Status::RE(0)),
        }
    }
}

fn decode_request<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>(
    request: &Request,
) -> Result<rfc003::bob::SwapRequest<AL, BL, AA, BA>, Error> {
    let request_body: RequestBody<AL, BL> = request
        .get_body()
        .ok_or(Error::Missing)?
        .map_err(|_| Error::Invalid)?;

    Ok(rfc003::bob::SwapRequest {
        alpha_asset: AA::from_bam_header(
            request
                .get_header("alpha_asset")
                .ok_or(Error::Missing)?
                .map_err(|_| Error::Invalid)?,
        )
        .map_err(|_| Error::Invalid)?,
        beta_asset: BA::from_bam_header(
            request
                .get_header("beta_asset")
                .ok_or(Error::Missing)?
                .map_err(|_| Error::Invalid)?,
        )
        .map_err(|_| Error::Invalid)?,
        alpha_ledger: AL::from_bam_header(
            request
                .get_header("alpha_ledger")
                .ok_or(Error::Missing)?
                .map_err(|_| Error::Invalid)?,
        )
        .map_err(|_| Error::Invalid)?,
        beta_ledger: BL::from_bam_header(
            request
                .get_header("beta_ledger")
                .ok_or(Error::Missing)?
                .map_err(|_| Error::Invalid)?,
        )
        .map_err(|_| Error::Invalid)?,
        alpha_ledger_refund_identity: request_body.alpha_ledger_refund_identity,
        beta_ledger_success_identity: request_body.beta_ledger_success_identity,
        alpha_ledger_lock_duration: request_body.alpha_ledger_lock_duration,
        secret_hash: request_body.secret_hash,
    })
}

enum Error {
    Missing,
    Invalid,
}
