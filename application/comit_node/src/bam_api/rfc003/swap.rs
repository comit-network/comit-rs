use crate::{
    bam_api::header::FromBamHeader,
    comit_client::{self, rfc003::RequestBody, SwapReject},
    swap_protocols::{
        asset::Asset,
        ledger::{Bitcoin, Ethereum},
        rfc003::{self, bob::BobSpawner, state_machine::StateMachineResponse, Ledger},
        SwapId, SwapProtocols,
    },
};
use bam::{
    config::Config,
    json::{Request, Response},
    Status,
};
use bitcoin_support::BitcoinQuantity;
use ethereum_support::{Erc20Quantity, EtherQuantity};
use futures::future::{self, Future};
use std::sync::Arc;

pub fn swap_config<B: BobSpawner>(bob_spawner: Arc<B>) -> Config<Request, Response> {
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

                    if let Ok(swap_request) =
                        decode_request::<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity>(
                            &request,
                        )
                    {
                        let response_future = match bob_spawner.spawn(swap_id, swap_request) {
                            Ok(response_future) => response_future,
                            Err(e) => {
                                error!("Unable to spawn Bob: {:?}", e);
                                return Box::new(future::ok(Response::new(Status::RE(0))));
                            }
                        };

                        Box::new(response_future.then(move |result| match result {
                            Ok(response) => Ok(to_bam_response::<Bitcoin, Ethereum>(response)),
                            Err(_) => {
                                warn!(
                                    "Failed to receive from oneshot channel for swap {}",
                                    swap_id
                                );
                                Ok(Response::new(Status::SE(0)))
                            }
                        }))
                    } else if let Ok(swap_request) =
                        decode_request::<Bitcoin, Ethereum, BitcoinQuantity, Erc20Quantity>(
                            &request,
                        )
                    {
                        let response_future = match bob_spawner.spawn(swap_id, swap_request) {
                            Ok(response_future) => response_future,
                            Err(e) => {
                                error!("Unable to spawn Bob: {:?}", e);
                                return Box::new(future::ok(Response::new(Status::RE(0))));
                            }
                        };

                        Box::new(response_future.then(move |result| match result {
                            Ok(response) => Ok(to_bam_response::<Bitcoin, Ethereum>(response)),
                            Err(_) => {
                                warn!(
                                    "Failed to receive from oneshot channel for swap {}",
                                    swap_id
                                );
                                Ok(Response::new(Status::SE(0)))
                            }
                        }))
                    } else if let Ok(swap_request) =
                        decode_request::<Ethereum, Bitcoin, EtherQuantity, BitcoinQuantity>(
                            &request,
                        )
                    {
                        let response_future = match bob_spawner.spawn(swap_id, swap_request) {
                            Ok(response_future) => response_future,
                            Err(e) => {
                                error!("Unable to spawn Bob: {:?}", e);
                                return Box::new(future::ok(Response::new(Status::RE(0))));
                            }
                        };

                        Box::new(response_future.then(move |result| match result {
                            Ok(response) => Ok(to_bam_response::<Ethereum, Bitcoin>(response)),
                            Err(_) => {
                                warn!(
                                    "Failed to receive from oneshot channel for swap {}",
                                    swap_id
                                );
                                Ok(Response::new(Status::SE(0)))
                            }
                        }))
                    } else {
                        // TODO: Specify and implement response code
                        Box::new(future::ok(Response::new(Status::SE(0))))
                    }
                }
            }
        },
    )
}

#[allow(clippy::type_complexity)]
fn to_bam_response<AL: Ledger, BL: Ledger>(
    result: Result<
        StateMachineResponse<AL::HtlcIdentity, BL::HtlcIdentity, BL::LockDuration>,
        SwapReject,
    >,
) -> Response {
    match result {
        Ok(response) => {
            Response::new(Status::OK(20)).with_body(comit_client::rfc003::AcceptResponseBody::<
                AL,
                BL,
            > {
                beta_ledger_refund_identity: response.beta_ledger_refund_identity.into(),
                alpha_ledger_redeem_identity: response.alpha_ledger_redeem_identity.into(),
                beta_ledger_lock_duration: response.beta_ledger_lock_duration,
            })
        }
        Err(SwapReject::Declined { reason: None }) => Response::new(Status::SE(20)),
        Err(SwapReject::Declined {
            reason: Some(reason),
        }) => Response::new(Status::SE(20)).with_header("REASON", reason),
        Err(_) => Response::new(Status::RE(0)),
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
        beta_ledger_redeem_identity: request_body.beta_ledger_redeem_identity,
        alpha_ledger_lock_duration: request_body.alpha_ledger_lock_duration,
        secret_hash: request_body.secret_hash,
    })
}

enum Error {
    Missing,
    Invalid,
}
