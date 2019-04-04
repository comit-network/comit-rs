use crate::{
    bam_ext::{FromBamHeader, ToBamHeader},
    comit_client::SwapReject,
    swap_protocols::{
        asset::{Asset, AssetKind},
        ledger::LedgerKind,
        rfc003::{self, bob::BobSpawner, CreateLedgerEvents, Ledger},
        LedgerEventDependencies, SwapId, SwapProtocol,
    },
};
use bam::{
    config::Config,
    json::{Response, ValidatedIncomingRequest},
    Status,
};
use futures::future::{self, Future};

pub fn swap_config<B: BobSpawner>(bob_spawner: B) -> Config<ValidatedIncomingRequest, Response> {
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
            let protocol: SwapProtocol = header!(request
                .take_header("swap_protocol")
                .map(SwapProtocol::from_bam_header));
            match protocol {
                SwapProtocol::Rfc003 => {
                    let swap_id = SwapId::default();

                    let alpha_ledger = header!(request
                        .take_header("alpha_ledger")
                        .map(LedgerKind::from_bam_header));
                    let beta_ledger = header!(request
                        .take_header("beta_ledger")
                        .map(LedgerKind::from_bam_header));
                    let alpha_asset = header!(request
                        .take_header("alpha_asset")
                        .map(AssetKind::from_bam_header));
                    let beta_asset = header!(request
                        .take_header("beta_asset")
                        .map(AssetKind::from_bam_header));

                    match (alpha_ledger, beta_ledger, alpha_asset, beta_asset) {
                        (
                            LedgerKind::Bitcoin(alpha_ledger),
                            LedgerKind::Ethereum(beta_ledger),
                            AssetKind::Bitcoin(alpha_asset),
                            AssetKind::Ether(beta_asset),
                        ) => handle_request(
                            &bob_spawner,
                            swap_id,
                            alpha_ledger,
                            beta_ledger,
                            alpha_asset,
                            beta_asset,
                            body!(request.take_body_as()),
                        ),
                        (
                            LedgerKind::Ethereum(alpha_ledger),
                            LedgerKind::Bitcoin(beta_ledger),
                            AssetKind::Ether(alpha_asset),
                            AssetKind::Bitcoin(beta_asset),
                        ) => handle_request(
                            &bob_spawner,
                            swap_id,
                            alpha_ledger,
                            beta_ledger,
                            alpha_asset,
                            beta_asset,
                            body!(request.take_body_as()),
                        ),
                        (
                            LedgerKind::Bitcoin(alpha_ledger),
                            LedgerKind::Ethereum(beta_ledger),
                            AssetKind::Bitcoin(alpha_asset),
                            AssetKind::Erc20(beta_asset),
                        ) => handle_request(
                            &bob_spawner,
                            swap_id,
                            alpha_ledger,
                            beta_ledger,
                            alpha_asset,
                            beta_asset,
                            body!(request.take_body_as()),
                        ),
                        (
                            LedgerKind::Ethereum(alpha_ledger),
                            LedgerKind::Bitcoin(beta_ledger),
                            AssetKind::Erc20(alpha_asset),
                            AssetKind::Bitcoin(beta_asset),
                        ) => handle_request(
                            &bob_spawner,
                            swap_id,
                            alpha_ledger,
                            beta_ledger,
                            alpha_asset,
                            beta_asset,
                            body!(request.take_body_as()),
                        ),
                        (alpha_ledger, beta_ledger, alpha_asset, beta_asset) => {
                            warn!(
                                "swapping {:?} to {:?} from {:?} to {:?} is currently not supported", alpha_asset, beta_asset, alpha_ledger, beta_ledger
                            );
                            Box::new(future::ok(Response::new(Status::RE(21))))
                        }
                    }
                }
                SwapProtocol::Unknown(protocol) => {
                    warn!(
                        "the swap protocol {} is currently not supported", protocol
                    );
                    Box::new(future::ok(Response::new(Status::RE(21))))
                },
            }
        },
    )
}

fn handle_request<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset, B: BobSpawner>(
    bob_spawner: &B,
    swap_id: SwapId,
    alpha_ledger: AL,
    beta_ledger: BL,
    alpha_asset: AA,
    beta_asset: BA,
    body: rfc003::messages::RequestBody<AL, BL>,
) -> Box<dyn Future<Item = Response, Error = ()> + Send + 'static>
where
    LedgerEventDependencies: CreateLedgerEvents<AL, AA> + CreateLedgerEvents<BL, BA>,
{
    match bob_spawner.spawn(
        swap_id,
        rfc003::messages::Request::<AL, BL, AA, BA> {
            alpha_asset,
            beta_asset,
            alpha_ledger,
            beta_ledger,
            alpha_ledger_refund_identity: body.alpha_ledger_refund_identity,
            beta_ledger_redeem_identity: body.beta_ledger_redeem_identity,
            alpha_expiry: body.alpha_expiry,
            beta_expiry: body.beta_expiry,
            secret_hash: body.secret_hash,
        },
    ) {
        Ok(response_future) => Box::new(response_future.then(move |result| {
            let response = match result {
                Ok(Ok(response)) => {
                    let body = rfc003::messages::AcceptResponseBody::<AL, BL> {
                        beta_ledger_refund_identity: response.beta_ledger_refund_identity,
                        alpha_ledger_redeem_identity: response.alpha_ledger_redeem_identity,
                    };
                    Response::new(Status::OK(20)).with_body(
                        serde_json::to_value(body)
                            .expect("body should always serialize into serde_json::Value"),
                    )
                }
                // FIXME: the called code should not be able to produce a "Rejected" here
                // Rejected is for cases were we can automatically determine that a given swap
                // request cannot be processes. As soon as we can dispatch the
                // request (and therefore these branches here are activated), the
                // only valid negative outcome should be "Declined"
                //
                // As long as Alice and Bob use the same state machine, this is not possible though.
                Ok(Err(SwapReject::Rejected)) => Response::new(Status::SE(21)),
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
