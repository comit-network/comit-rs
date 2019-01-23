use crate::{
    bam_ext::{FromBamHeader, ToBamHeader},
    comit_client::{self, rfc003::RequestBody, SwapReject},
    swap_protocols::{
        asset::{Asset, Assets},
        ledger::Ledgers,
        rfc003::{self, bob::BobSpawner, CreateLedgerEvents, Ledger},
        LedgerEventDependencies, SwapId, SwapProtocols,
    },
};
use bam::{
    config::Config,
    json::{Response, ValidatedIncomingRequest},
    Status,
};
use futures::future::{self, Future};
use std::sync::Arc;

pub fn swap_config<B: BobSpawner>(
    bob_spawner: Arc<B>,
) -> Config<ValidatedIncomingRequest, Response> {
    Config::default().on_request(
        "SWAP",
        &[
            "beta_ledger",
            "alpha_ledger",
            "beta_asset",
            "alpha_asset",
            "swap_protocol",
        ],
        move |mut request: ValidatedIncomingRequest| {
            let protocol: SwapProtocols = header!(request
                .take_header("swap_protocol")
                .map(SwapProtocols::from_bam_header));
            match protocol {
                SwapProtocols::Rfc003 => {
                    let swap_id = SwapId::default();

                    let alpha_ledger = header!(request
                        .take_header("alpha_ledger")
                        .map(Ledgers::from_bam_header));
                    let beta_ledger = header!(request
                        .take_header("beta_ledger")
                        .map(Ledgers::from_bam_header));
                    let alpha_asset = header!(request
                        .take_header("alpha_asset")
                        .map(Assets::from_bam_header));
                    let beta_asset = header!(request
                        .take_header("beta_asset")
                        .map(Assets::from_bam_header));

                    match (alpha_ledger, beta_ledger, alpha_asset, beta_asset) {
                        (
                            Ledgers::Bitcoin(alpha_ledger),
                            Ledgers::Ethereum(beta_ledger),
                            Assets::Bitcoin(alpha_asset),
                            Assets::Ether(beta_asset),
                        ) => handle_request(
                            Arc::clone(&bob_spawner),
                            swap_id,
                            alpha_ledger,
                            beta_ledger,
                            alpha_asset,
                            beta_asset,
                            body!(request.take_body_as()),
                        ),
                        (
                            Ledgers::Ethereum(alpha_ledger),
                            Ledgers::Bitcoin(beta_ledger),
                            Assets::Ether(alpha_asset),
                            Assets::Bitcoin(beta_asset),
                        ) => handle_request(
                            Arc::clone(&bob_spawner),
                            swap_id,
                            alpha_ledger,
                            beta_ledger,
                            alpha_asset,
                            beta_asset,
                            body!(request.take_body_as()),
                        ),
                        (
                            Ledgers::Bitcoin(alpha_ledger),
                            Ledgers::Ethereum(beta_ledger),
                            Assets::Bitcoin(alpha_asset),
                            Assets::Erc20(beta_asset),
                        ) => handle_request(
                            Arc::clone(&bob_spawner),
                            swap_id,
                            alpha_ledger,
                            beta_ledger,
                            alpha_asset,
                            beta_asset,
                            body!(request.take_body_as()),
                        ),
                        (
                            Ledgers::Ethereum(alpha_ledger),
                            Ledgers::Bitcoin(beta_ledger),
                            Assets::Erc20(alpha_asset),
                            Assets::Bitcoin(beta_asset),
                        ) => handle_request(
                            Arc::clone(&bob_spawner),
                            swap_id,
                            alpha_ledger,
                            beta_ledger,
                            alpha_asset,
                            beta_asset,
                            body!(request.take_body_as()),
                        ),
                        _ => unimplemented!(),
                    }
                }
                SwapProtocols::Unknown { .. } => unimplemented!(),
            }
        },
    )
}

fn handle_request<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset, B: BobSpawner>(
    bob_spawner: Arc<B>,
    swap_id: SwapId,
    alpha_ledger: AL,
    beta_ledger: BL,
    alpha_asset: AA,
    beta_asset: BA,
    body: RequestBody<AL, BL>,
) -> Box<dyn Future<Item = Response, Error = ()> + Send + 'static>
where
    LedgerEventDependencies: CreateLedgerEvents<AL, AA> + CreateLedgerEvents<BL, BA>,
{
    match bob_spawner.spawn(
        swap_id,
        rfc003::bob::SwapRequest::<AL, BL, AA, BA> {
            alpha_asset,
            beta_asset,
            alpha_ledger,
            beta_ledger,
            alpha_ledger_refund_identity: body.alpha_ledger_refund_identity,
            beta_ledger_redeem_identity: body.beta_ledger_redeem_identity,
            alpha_ledger_lock_duration: body.alpha_ledger_lock_duration,
            secret_hash: body.secret_hash,
        },
    ) {
        Ok(response_future) => Box::new(response_future.then(move |result| {
            let response = match result {
                Ok(Ok(response)) => {
                    let body = comit_client::rfc003::AcceptResponseBody::<AL, BL> {
                        beta_ledger_refund_identity: response.beta_ledger_refund_identity.into(),
                        alpha_ledger_redeem_identity: response.alpha_ledger_redeem_identity.into(),
                        beta_ledger_lock_duration: response.beta_ledger_lock_duration,
                    };
                    Response::new(Status::OK(20)).with_body(
                        serde_json::to_value(body)
                            .expect("body should always serialize into serde_json::Value"),
                    )
                }
                Ok(Err(SwapReject::Rejected)) => unimplemented!(),
                Ok(Err(SwapReject::Declined { reason: None })) => Response::new(Status::SE(20)),
                Ok(Err(SwapReject::Declined {
                    reason: Some(reason),
                })) => Response::new(Status::SE(20)).with_header(
                    "REASON",
                    reason
                        .to_bam_header()
                        .expect("reason header shouldn't fail to serialize"),
                ),
                Err(_) => {
                    warn!(
                        "Failed to receive from oneshot channel for swap {}",
                        swap_id
                    );
                    Response::new(Status::SE(0))
                }
            };

            Ok(response)
        })),
        Err(e) => {
            error!("Unable to spawn Bob: {:?}", e);
            Box::new(future::ok(Response::new(Status::RE(0))))
        }
    }
}
