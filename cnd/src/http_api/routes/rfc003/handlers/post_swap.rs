use crate::{
    db::{Save, Saver, Swap},
    ethereum::{self, Erc20Token, EtherQuantity},
    http_api::{HttpAsset, HttpLedger},
    network::{DialInformation, Network},
    seed::SwapSeed,
    swap_protocols::{
        self,
        asset::Asset,
        ledger::{self, Bitcoin, Ethereum},
        rfc003::{
            self, alice::State, events::HtlcEvents, state_store::StateStore, Accept, Decline,
            Ledger, Request, SecretHash, SecretSource,
        },
        HashFunction, Role, SwapId,
    },
    timestamp::Timestamp,
};
use anyhow::Context;
use bitcoin::Amount;
use futures::Future;
use futures_core::{
    compat::Future01CompatExt,
    future::{FutureExt, TryFutureExt},
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tokio::executor::Executor;

pub async fn handle_post_swap<
    D: Clone
        + Executor
        + StateStore
        + Save<Swap>
        + SwapSeed
        + Saver
        + Network
        + Clone
        + HtlcEvents<Bitcoin, Amount>
        + HtlcEvents<Ethereum, EtherQuantity>
        + HtlcEvents<Ethereum, Erc20Token>,
>(
    dependencies: D,
    body: serde_json::Value,
) -> anyhow::Result<SwapCreated> {
    let id = SwapId::default();
    let seed = dependencies.swap_seed(id);
    let secret_hash = seed.secret().hash();

    let body = serde_json::from_value(body)?;

    match body {
        SwapRequestBody {
            alpha_ledger: HttpLedger::Bitcoin(alpha_ledger),
            beta_ledger: HttpLedger::Ethereum(beta_ledger),
            alpha_asset: HttpAsset::Bitcoin(alpha_asset),
            beta_asset: HttpAsset::Ether(beta_asset),
            alpha_expiry,
            beta_expiry,
            identities,
            peer,
        } => {
            let identities = identities.into_identities(&seed)?;
            let request = new_request(
                id,
                alpha_ledger,
                beta_ledger,
                alpha_asset,
                beta_asset,
                alpha_expiry,
                beta_expiry,
                identities,
                secret_hash,
            );
            initiate_request(dependencies, id, peer, request).await?;
        }
        SwapRequestBody {
            alpha_ledger: HttpLedger::Ethereum(alpha_ledger),
            beta_ledger: HttpLedger::Bitcoin(beta_ledger),
            alpha_asset: HttpAsset::Ether(alpha_asset),
            beta_asset: HttpAsset::Bitcoin(beta_asset),
            alpha_expiry,
            beta_expiry,
            identities,
            peer,
        } => {
            let identities = identities.into_identities(&seed)?;
            let request = new_request(
                id,
                alpha_ledger,
                beta_ledger,
                alpha_asset,
                beta_asset,
                alpha_expiry,
                beta_expiry,
                identities,
                secret_hash,
            );
            initiate_request(dependencies, id, peer, request).await?;
        }
        SwapRequestBody {
            alpha_ledger: HttpLedger::Bitcoin(alpha_ledger),
            beta_ledger: HttpLedger::Ethereum(beta_ledger),
            alpha_asset: HttpAsset::Bitcoin(alpha_asset),
            beta_asset: HttpAsset::Erc20(beta_asset),
            alpha_expiry,
            beta_expiry,
            identities,
            peer,
        } => {
            let identities = identities.into_identities(&seed)?;
            let request = new_request(
                id,
                alpha_ledger,
                beta_ledger,
                alpha_asset,
                beta_asset,
                alpha_expiry,
                beta_expiry,
                identities,
                secret_hash,
            );
            initiate_request(dependencies, id, peer, request).await?;
        }
        SwapRequestBody {
            alpha_ledger: HttpLedger::Ethereum(alpha_ledger),
            beta_ledger: HttpLedger::Bitcoin(beta_ledger),
            alpha_asset: HttpAsset::Erc20(alpha_asset),
            beta_asset: HttpAsset::Bitcoin(beta_asset),
            alpha_expiry,
            beta_expiry,
            identities,
            peer,
        } => {
            let identities = identities.into_identities(&seed)?;
            let request = new_request(
                id,
                alpha_ledger,
                beta_ledger,
                alpha_asset,
                beta_asset,
                alpha_expiry,
                beta_expiry,
                identities,
                secret_hash,
            );
            initiate_request(dependencies, id, peer, request).await?;
        }
        _ => {
            return Err(anyhow::Error::from(UnsupportedSwap {
                alpha_ledger: body.alpha_ledger,
                beta_ledger: body.beta_ledger,
                alpha_asset: body.alpha_asset,
                beta_asset: body.beta_asset,
            }))
        }
    }

    Ok(SwapCreated { id })
}

#[allow(clippy::too_many_arguments)]
fn new_request<AL, BL, AA, BA>(
    id: SwapId,
    alpha_ledger: AL,
    beta_ledger: BL,
    alpha_asset: AA,
    beta_asset: BA,
    alpha_expiry: Option<Timestamp>,
    beta_expiry: Option<Timestamp>,
    identities: Identities<AL, BL>,
    secret_hash: SecretHash,
) -> rfc003::Request<AL, BL, AA, BA>
where
    AL: Ledger,
    BL: Ledger,
    AA: Asset,
    BA: Asset,
{
    rfc003::Request {
        swap_id: id,
        alpha_ledger,
        beta_ledger,
        alpha_asset,
        beta_asset,
        hash_function: HashFunction::Sha256,
        alpha_ledger_refund_identity: identities.alpha_ledger_refund_identity,
        beta_ledger_redeem_identity: identities.beta_ledger_redeem_identity,
        alpha_expiry: alpha_expiry.unwrap_or_else(default_alpha_expiry),
        beta_expiry: beta_expiry.unwrap_or_else(default_beta_expiry),
        secret_hash,
    }
}

/// An error type for describing that a particular combination of assets and
/// ledgers is not supported.
#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("swapping {alpha_asset:?} for {beta_asset:?} from {alpha_ledger:?} to {beta_ledger:?} is not supported")]
pub struct UnsupportedSwap {
    alpha_asset: HttpAsset,
    beta_asset: HttpAsset,
    alpha_ledger: HttpLedger,
    beta_ledger: HttpLedger,
}

async fn initiate_request<D, AL, BL, AA, BA>(
    dependencies: D,
    id: SwapId,
    peer: DialInformation,
    swap_request: rfc003::Request<AL, BL, AA, BA>,
) -> anyhow::Result<()>
where
    D: StateStore
        + Executor
        + SwapSeed
        + Save<Request<AL, BL, AA, BA>>
        + Save<Accept<AL, BL>>
        + Save<Swap>
        + Save<Decline>
        + Network
        + HtlcEvents<AL, AA>
        + HtlcEvents<BL, BA>
        + Clone,
    AL: Ledger,
    BL: Ledger,
    AA: Asset,
    BA: Asset,
{
    let counterparty = peer.peer_id.clone();
    let seed = dependencies.swap_seed(id);

    Save::save(&dependencies, Swap::new(id, Role::Alice, counterparty)).await?;
    Save::save(&dependencies, swap_request.clone()).await?;

    let state = State::proposed(swap_request.clone(), seed);
    StateStore::insert(&dependencies, id, state);

    let future = {
        async move {
            let response = dependencies
                .send_request(peer.clone(), swap_request.clone())
                .compat()
                .await
                .with_context(|| format!("Failed to send swap request to {}", peer.clone()))?;

            match response {
                Ok(accept) => {
                    Save::save(&dependencies, accept).await?;

                    swap_protocols::init_accepted_swap(
                        &dependencies,
                        swap_request,
                        accept,
                        Role::Alice,
                    )?;
                }
                Err(decline) => {
                    log::info!("Swap declined: {:?}", decline);
                    let state = State::declined(swap_request.clone(), decline, seed);
                    StateStore::insert(&dependencies, id, state.clone());
                    Save::save(&dependencies, decline).await?;
                }
            };
            Ok(())
        }
    };
    tokio::spawn(future.boxed().compat().map_err(|e: anyhow::Error| {
        log::error!("{:?}", e);
    }));
    Ok(())
}

#[derive(Serialize, Clone, Copy, Debug)]
pub struct SwapCreated {
    pub id: SwapId,
}

/// A struct describing the expected HTTP body for creating a new swap request.
///
/// To achieve the deserialization we need for this usecase, we make use of a
/// lot of serde features. Check the documentation of the types used in this
/// struct for more details.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct SwapRequestBody {
    alpha_asset: HttpAsset,
    beta_asset: HttpAsset,
    alpha_ledger: HttpLedger,
    beta_ledger: HttpLedger,
    alpha_expiry: Option<Timestamp>,
    beta_expiry: Option<Timestamp>,
    #[serde(flatten)]
    identities: HttpIdentities,
    peer: DialInformation,
}

/// The identities a user may have to provide for a given swap.
///
/// To make the implementation easier, this is hardcoded to Ethereum addresses
/// for now because those are always provided upfront.
#[derive(Clone, Debug, Deserialize, PartialEq)]
struct HttpIdentities {
    alpha_ledger_refund_identity: Option<ethereum::Address>,
    beta_ledger_redeem_identity: Option<ethereum::Address>,
}

#[derive(Debug, Clone)]
struct Identities<AL: Ledger, BL: Ledger> {
    pub alpha_ledger_refund_identity: AL::Identity,
    pub beta_ledger_redeem_identity: BL::Identity,
}

trait IntoIdentities<AL: Ledger, BL: Ledger> {
    fn into_identities(
        self,
        secret_source: &dyn SecretSource,
    ) -> anyhow::Result<Identities<AL, BL>>;
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("{kind} identity was not expected")]
pub struct UnexpectedIdentity {
    kind: IdentityKind,
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("{kind} identity was missing")]
pub struct MissingIdentity {
    kind: IdentityKind,
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("{kind} was not a valid ethereum address")]
pub struct InvalidEthereumAddress {
    kind: IdentityKind,
    source: <ethereum::Address as FromStr>::Err,
}

#[derive(strum_macros::Display, Debug, Clone, Copy)]
#[strum(serialize_all = "snake_case")]
pub enum IdentityKind {
    AlphaLedgerRefundIdentity,
    BetaLedgerRedeemIdentity,
}

impl IntoIdentities<ledger::Bitcoin, ledger::Ethereum> for HttpIdentities {
    fn into_identities(
        self,
        secret_source: &dyn SecretSource,
    ) -> anyhow::Result<Identities<ledger::Bitcoin, ledger::Ethereum>> {
        let HttpIdentities {
            alpha_ledger_refund_identity,
            beta_ledger_redeem_identity,
        } = self;

        let beta_ledger_redeem_identity =
            match (alpha_ledger_refund_identity, beta_ledger_redeem_identity) {
                (None, Some(beta_ledger_redeem_identity)) => beta_ledger_redeem_identity,
                (_, None) => {
                    return Err(anyhow::Error::from(MissingIdentity {
                        kind: IdentityKind::BetaLedgerRedeemIdentity,
                    }))
                }
                (Some(_), _) => {
                    return Err(anyhow::Error::from(UnexpectedIdentity {
                        kind: IdentityKind::AlphaLedgerRefundIdentity,
                    }))
                }
            };

        let alpha_ledger_refund_identity = crate::bitcoin::PublicKey::from_secret_key(
            &*crate::SECP,
            &secret_source.secp256k1_refund(),
        );

        Ok(Identities {
            alpha_ledger_refund_identity,
            beta_ledger_redeem_identity,
        })
    }
}

impl IntoIdentities<ledger::Ethereum, ledger::Bitcoin> for HttpIdentities {
    fn into_identities(
        self,
        secret_source: &dyn SecretSource,
    ) -> anyhow::Result<Identities<ledger::Ethereum, ledger::Bitcoin>> {
        let HttpIdentities {
            alpha_ledger_refund_identity,
            beta_ledger_redeem_identity,
        } = self;

        let alpha_ledger_refund_identity =
            match (alpha_ledger_refund_identity, beta_ledger_redeem_identity) {
                (Some(alpha_ledger_refund_identity), None) => alpha_ledger_refund_identity,
                (_, Some(_)) => {
                    return Err(anyhow::Error::from(UnexpectedIdentity {
                        kind: IdentityKind::BetaLedgerRedeemIdentity,
                    }))
                }
                (None, _) => {
                    return Err(anyhow::Error::from(MissingIdentity {
                        kind: IdentityKind::AlphaLedgerRefundIdentity,
                    }))
                }
            };

        let beta_ledger_redeem_identity = crate::bitcoin::PublicKey::from_secret_key(
            &*crate::SECP,
            &secret_source.secp256k1_redeem(),
        );

        Ok(Identities {
            alpha_ledger_refund_identity,
            beta_ledger_redeem_identity,
        })
    }
}

fn default_alpha_expiry() -> Timestamp {
    Timestamp::now().plus(60 * 60 * 24)
}

fn default_beta_expiry() -> Timestamp {
    Timestamp::now().plus(60 * 60 * 12)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{network::DialInformation, swap_protocols::ledger::ethereum::ChainId};
    use spectral::prelude::*;

    #[test]
    fn can_deserialize_swap_request_body() {
        let body = r#"{
                "alpha_ledger": {
                    "name": "bitcoin",
                    "network": "regtest"
                },
                "beta_ledger": {
                    "name": "ethereum",
                    "network": "regtest"
                },
                "alpha_asset": {
                    "name": "bitcoin",
                    "quantity": "100000000"
                },
                "beta_asset": {
                    "name": "ether",
                    "quantity": "10000000000000000000"
                },
                "beta_ledger_redeem_identity": "0x00a329c0648769a73afac7f9381e08fb43dbea72",
                "alpha_expiry": 2000000000,
                "beta_expiry": 2000000000,
                "peer": "Qma9T5YraSnpRDZqRR4krcSJabThc8nwZuJV3LercPHufi"
            }"#;

        let body = serde_json::from_str::<SwapRequestBody>(body);

        assert_that(&body).is_ok();
    }

    #[test]
    fn given_peer_id_with_address_can_deserialize_swap_request_body() {
        let body = r#"{
                "alpha_ledger": {
                    "name": "bitcoin",
                    "network": "regtest"
                },
                "beta_ledger": {
                    "name": "ethereum",
                    "network": "regtest"
                },
                "alpha_asset": {
                    "name": "bitcoin",
                    "quantity": "100000000"
                },
                "beta_asset": {
                    "name": "ether",
                    "quantity": "10000000000000000000"
                },
                "beta_ledger_redeem_identity": "0x00a329c0648769a73afac7f9381e08fb43dbea72",
                "alpha_expiry": 2000000000,
                "beta_expiry": 2000000000,
                "peer": { "peer_id": "Qma9T5YraSnpRDZqRR4krcSJabThc8nwZuJV3LercPHufi", "address_hint": "/ip4/8.9.0.1/tcp/9999" }
            }"#;

        let body = serde_json::from_str::<SwapRequestBody>(body);

        assert_that(&body)
            .is_ok()
            .map(|b| &b.peer)
            .is_equal_to(&DialInformation {
                peer_id: "Qma9T5YraSnpRDZqRR4krcSJabThc8nwZuJV3LercPHufi"
                    .parse()
                    .unwrap(),
                address_hint: Some("/ip4/8.9.0.1/tcp/9999".parse().unwrap()),
            });
    }

    #[test]
    fn can_deserialize_swap_request_body_with_chain_id() {
        let body = r#"{
                "alpha_ledger": {
                    "name": "bitcoin",
                    "network": "regtest"
                },
                "beta_ledger": {
                    "name": "ethereum",
                    "chain_id": 3
                },
                "alpha_asset": {
                    "name": "bitcoin",
                    "quantity": "100000000"
                },
                "beta_asset": {
                    "name": "ether",
                    "quantity": "10000000000000000000"
                },
                "beta_ledger_redeem_identity": "0x00a329c0648769a73afac7f9381e08fb43dbea72",
                "alpha_expiry": 2000000000,
                "beta_expiry": 2000000000,
                "peer": "Qma9T5YraSnpRDZqRR4krcSJabThc8nwZuJV3LercPHufi"
            }"#;

        let body = serde_json::from_str::<SwapRequestBody>(body);

        assert_that(&body)
            .is_ok()
            .map(|b| &b.beta_ledger)
            .is_equal_to(&HttpLedger::Ethereum(ledger::Ethereum {
                chain_id: ChainId::new(3),
            }));
    }
}
