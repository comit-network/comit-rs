use crate::{
    db::{LoadAcceptedSwap, Save, Sqlite, Swap},
    htlc_location,
    http_api::{HttpAsset, HttpLedger},
    identity,
    init_swap::init_accepted_swap,
    network::{DialInformation, SendRequest},
    seed::DeriveSwapSeed,
    swap_protocols::{
        rfc003::{
            self, alice,
            events::{HtlcDeployed, HtlcFunded, HtlcRedeemed, HtlcRefunded},
            Accept, Decline, DeriveIdentities, DeriveSecret, Request, SecretHash,
        },
        state_store::Insert,
        Facade, HashFunction, Role, SwapId,
    },
    timestamp::Timestamp,
    transaction,
};
use anyhow::Context;
use futures::future::TryFutureExt;
use libp2p_comit::frame::OutboundRequest;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{convert::TryInto, fmt::Debug, str::FromStr};

async fn initiate_request<AL, BL, AA, BA, AH, BH, AI, BI, AT, BT>(
    dependencies: Facade,
    id: SwapId,
    peer: DialInformation,
    swap_request: rfc003::Request<AL, BL, AA, BA, AI, BI>,
) -> anyhow::Result<()>
where
    Sqlite:
        Save<Request<AL, BL, AA, BA, AI, BI>> + Save<Accept<AI, BI>> + Save<Swap> + Save<Decline>,
    AL: Clone + Send + Sync + 'static,
    BL: Clone + Send + Sync + 'static,
    AA: Clone + Ord + Send + Sync + 'static,
    BA: Clone + Ord + Send + Sync + 'static,
    AH: Clone + Send + Sync + 'static,
    BH: Clone + Send + Sync + 'static,
    AI: Clone + Send + Sync + 'static,
    BI: Clone + Send + Sync + 'static,
    AT: Clone + Send + Sync + 'static,
    BT: Clone + Send + Sync + 'static,
    rfc003::messages::AcceptResponseBody<AI, BI>: DeserializeOwned,
    Accept<AI, BI>: Copy,
    rfc003::Request<AL, BL, AA, BA, AI, BI>: TryInto<OutboundRequest> + Clone,
    <rfc003::Request<AL, BL, AA, BA, AI, BI> as TryInto<OutboundRequest>>::Error: Debug,
    Facade: LoadAcceptedSwap<AL, BL, AA, BA, AI, BI>
        + HtlcFunded<AL, AA, AH, AI, AT>
        + HtlcFunded<BL, BA, BH, BI, BT>
        + HtlcDeployed<AL, AA, AH, AI, AT>
        + HtlcDeployed<BL, BA, BH, BI, BT>
        + HtlcRedeemed<AL, AA, AH, AI, AT>
        + HtlcRedeemed<BL, BA, BH, BI, BT>
        + HtlcRefunded<AL, AA, AH, AI, AT>
        + HtlcRefunded<BL, BA, BH, BI, BT>,
{
    tracing::trace!("initiating new request: {}", swap_request.swap_id);

    let counterparty = peer.peer_id.clone();
    let seed = dependencies.derive_swap_seed(id);

    Save::save(&dependencies, Swap::new(id, Role::Alice, counterparty)).await?;
    Save::save(&dependencies, swap_request.clone()).await?;

    let state =
        alice::State::<_, _, _, _, AH, BH, _, _, AT, BT>::proposed(swap_request.clone(), seed);
    dependencies.insert(id, state);

    let future = {
        async move {
            let response = dependencies
                .send_request(peer.clone(), swap_request.clone())
                .await
                .with_context(|| format!("Failed to send swap request to {}", peer.clone()))?;

            match response {
                Ok(accept) => {
                    Save::save(&dependencies, accept).await?;
                    let accepted = LoadAcceptedSwap::<AL, BL, AA, BA, AI, BI>::load_accepted_swap(
                        &dependencies,
                        &id,
                    )
                    .await?;
                    init_accepted_swap::<_, _, _, _, _, AH, BH, _, _, AT, BT>(
                        &dependencies,
                        accepted,
                        Role::Alice,
                    )?;
                }
                Err(decline) => {
                    tracing::info!("Swap declined: {}", decline.swap_id);
                    let state = alice::State::<_, _, _, _, AH, BH, _, _, AT, BT>::declined(
                        swap_request.clone(),
                        decline,
                        seed,
                    );
                    dependencies.insert(id, state);
                    Save::save(&dependencies, decline).await?;
                }
            };
            Ok(())
        }
    };

    tokio::task::spawn(future.map_err(|e: anyhow::Error| {
        tracing::error!("{}", e);
    }));

    Ok(())
}

pub async fn handle_post_swap(
    dependencies: Facade,
    body: serde_json::Value,
) -> anyhow::Result<SwapCreated> {
    let id = SwapId::default();
    let seed = dependencies.derive_swap_seed(id);
    let secret_hash = seed.derive_secret().hash();

    let body = serde_json::from_value(body)?;

    match body {
        SwapRequestBody {
            alpha_ledger: HttpLedger::BitcoinMainnet(alpha_ledger),
            beta_ledger: HttpLedger::Ethereum(beta_ledger),
            alpha_asset: HttpAsset::Bitcoin(alpha_asset),
            beta_asset: HttpAsset::Ether(beta_asset),
            alpha_expiry,
            beta_expiry,
            identities,
            peer,
        } => {
            let identities = identities.into_bitcoin_ethereum_identities(&seed)?;
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
            initiate_request::<
                _,
                _,
                _,
                _,
                htlc_location::Bitcoin,
                htlc_location::Ethereum,
                _,
                _,
                transaction::Bitcoin,
                transaction::Ethereum,
            >(dependencies, id, peer, request)
            .await?;
        }
        SwapRequestBody {
            alpha_ledger: HttpLedger::BitcoinTestnet(alpha_ledger),
            beta_ledger: HttpLedger::Ethereum(beta_ledger),
            alpha_asset: HttpAsset::Bitcoin(alpha_asset),
            beta_asset: HttpAsset::Ether(beta_asset),
            alpha_expiry,
            beta_expiry,
            identities,
            peer,
        } => {
            let identities = identities.into_bitcoin_ethereum_identities(&seed)?;
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
            initiate_request::<
                _,
                _,
                _,
                _,
                htlc_location::Bitcoin,
                htlc_location::Ethereum,
                _,
                _,
                transaction::Bitcoin,
                transaction::Ethereum,
            >(dependencies, id, peer, request)
            .await?;
        }
        SwapRequestBody {
            alpha_ledger: HttpLedger::BitcoinRegtest(alpha_ledger),
            beta_ledger: HttpLedger::Ethereum(beta_ledger),
            alpha_asset: HttpAsset::Bitcoin(alpha_asset),
            beta_asset: HttpAsset::Ether(beta_asset),
            alpha_expiry,
            beta_expiry,
            identities,
            peer,
        } => {
            let identities = identities.into_bitcoin_ethereum_identities(&seed)?;
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
            initiate_request::<
                _,
                _,
                _,
                _,
                htlc_location::Bitcoin,
                htlc_location::Ethereum,
                _,
                _,
                transaction::Bitcoin,
                transaction::Ethereum,
            >(dependencies, id, peer, request)
            .await?;
        }
        SwapRequestBody {
            alpha_ledger: HttpLedger::Ethereum(alpha_ledger),
            beta_ledger: HttpLedger::BitcoinMainnet(beta_ledger),
            alpha_asset: HttpAsset::Ether(alpha_asset),
            beta_asset: HttpAsset::Bitcoin(beta_asset),
            alpha_expiry,
            beta_expiry,
            identities,
            peer,
        } => {
            let identities = identities.into_ethereum_bitcoin_identities(&seed)?;
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
            initiate_request::<
                _,
                _,
                _,
                _,
                htlc_location::Ethereum,
                htlc_location::Bitcoin,
                _,
                _,
                transaction::Ethereum,
                transaction::Bitcoin,
            >(dependencies, id, peer, request)
            .await?;
        }
        SwapRequestBody {
            alpha_ledger: HttpLedger::Ethereum(alpha_ledger),
            beta_ledger: HttpLedger::BitcoinTestnet(beta_ledger),
            alpha_asset: HttpAsset::Ether(alpha_asset),
            beta_asset: HttpAsset::Bitcoin(beta_asset),
            alpha_expiry,
            beta_expiry,
            identities,
            peer,
        } => {
            let identities = identities.into_ethereum_bitcoin_identities(&seed)?;
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
            initiate_request::<
                _,
                _,
                _,
                _,
                htlc_location::Ethereum,
                htlc_location::Bitcoin,
                _,
                _,
                transaction::Ethereum,
                transaction::Bitcoin,
            >(dependencies, id, peer, request)
            .await?;
        }
        SwapRequestBody {
            alpha_ledger: HttpLedger::Ethereum(alpha_ledger),
            beta_ledger: HttpLedger::BitcoinRegtest(beta_ledger),
            alpha_asset: HttpAsset::Ether(alpha_asset),
            beta_asset: HttpAsset::Bitcoin(beta_asset),
            alpha_expiry,
            beta_expiry,
            identities,
            peer,
        } => {
            let identities = identities.into_ethereum_bitcoin_identities(&seed)?;
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
            initiate_request::<
                _,
                _,
                _,
                _,
                htlc_location::Ethereum,
                htlc_location::Bitcoin,
                _,
                _,
                transaction::Ethereum,
                transaction::Bitcoin,
            >(dependencies, id, peer, request)
            .await?;
        }
        SwapRequestBody {
            alpha_ledger: HttpLedger::BitcoinMainnet(alpha_ledger),
            beta_ledger: HttpLedger::Ethereum(beta_ledger),
            alpha_asset: HttpAsset::Bitcoin(alpha_asset),
            beta_asset: HttpAsset::Erc20(beta_asset),
            alpha_expiry,
            beta_expiry,
            identities,
            peer,
        } => {
            let identities = identities.into_bitcoin_ethereum_identities(&seed)?;
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
            initiate_request::<
                _,
                _,
                _,
                _,
                htlc_location::Bitcoin,
                htlc_location::Ethereum,
                _,
                _,
                transaction::Bitcoin,
                transaction::Ethereum,
            >(dependencies, id, peer, request)
            .await?;
        }
        SwapRequestBody {
            alpha_ledger: HttpLedger::BitcoinTestnet(alpha_ledger),
            beta_ledger: HttpLedger::Ethereum(beta_ledger),
            alpha_asset: HttpAsset::Bitcoin(alpha_asset),
            beta_asset: HttpAsset::Erc20(beta_asset),
            alpha_expiry,
            beta_expiry,
            identities,
            peer,
        } => {
            let identities = identities.into_bitcoin_ethereum_identities(&seed)?;
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
            initiate_request::<
                _,
                _,
                _,
                _,
                htlc_location::Bitcoin,
                htlc_location::Ethereum,
                _,
                _,
                transaction::Bitcoin,
                transaction::Ethereum,
            >(dependencies, id, peer, request)
            .await?;
        }
        SwapRequestBody {
            alpha_ledger: HttpLedger::BitcoinRegtest(alpha_ledger),
            beta_ledger: HttpLedger::Ethereum(beta_ledger),
            alpha_asset: HttpAsset::Bitcoin(alpha_asset),
            beta_asset: HttpAsset::Erc20(beta_asset),
            alpha_expiry,
            beta_expiry,
            identities,
            peer,
        } => {
            let identities = identities.into_bitcoin_ethereum_identities(&seed)?;
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
            initiate_request::<
                _,
                _,
                _,
                _,
                htlc_location::Bitcoin,
                htlc_location::Ethereum,
                _,
                _,
                transaction::Bitcoin,
                transaction::Ethereum,
            >(dependencies, id, peer, request)
            .await?;
        }
        SwapRequestBody {
            alpha_ledger: HttpLedger::Ethereum(alpha_ledger),
            beta_ledger: HttpLedger::BitcoinMainnet(beta_ledger),
            alpha_asset: HttpAsset::Erc20(alpha_asset),
            beta_asset: HttpAsset::Bitcoin(beta_asset),
            alpha_expiry,
            beta_expiry,
            identities,
            peer,
        } => {
            let identities = identities.into_ethereum_bitcoin_identities(&seed)?;
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
            initiate_request::<
                _,
                _,
                _,
                _,
                htlc_location::Ethereum,
                htlc_location::Bitcoin,
                _,
                _,
                transaction::Ethereum,
                transaction::Bitcoin,
            >(dependencies, id, peer, request)
            .await?;
        }
        SwapRequestBody {
            alpha_ledger: HttpLedger::Ethereum(alpha_ledger),
            beta_ledger: HttpLedger::BitcoinTestnet(beta_ledger),
            alpha_asset: HttpAsset::Erc20(alpha_asset),
            beta_asset: HttpAsset::Bitcoin(beta_asset),
            alpha_expiry,
            beta_expiry,
            identities,
            peer,
        } => {
            let identities = identities.into_ethereum_bitcoin_identities(&seed)?;
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
            initiate_request::<
                _,
                _,
                _,
                _,
                htlc_location::Ethereum,
                htlc_location::Bitcoin,
                _,
                _,
                transaction::Ethereum,
                transaction::Bitcoin,
            >(dependencies, id, peer, request)
            .await?;
        }
        SwapRequestBody {
            alpha_ledger: HttpLedger::Ethereum(alpha_ledger),
            beta_ledger: HttpLedger::BitcoinRegtest(beta_ledger),
            alpha_asset: HttpAsset::Erc20(alpha_asset),
            beta_asset: HttpAsset::Bitcoin(beta_asset),
            alpha_expiry,
            beta_expiry,
            identities,
            peer,
        } => {
            let identities = identities.into_ethereum_bitcoin_identities(&seed)?;
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
            initiate_request::<
                _,
                _,
                _,
                _,
                htlc_location::Ethereum,
                htlc_location::Bitcoin,
                _,
                _,
                transaction::Ethereum,
                transaction::Bitcoin,
            >(dependencies, id, peer, request)
            .await?;
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
fn new_request<AL, BL, AA, BA, AI, BI>(
    id: SwapId,
    alpha_ledger: AL,
    beta_ledger: BL,
    alpha_asset: AA,
    beta_asset: BA,
    alpha_expiry: Option<Timestamp>,
    beta_expiry: Option<Timestamp>,
    identities: Identities<AI, BI>,
    secret_hash: SecretHash,
) -> rfc003::Request<AL, BL, AA, BA, AI, BI> {
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
#[derive(Debug, Clone, thiserror::Error)]
#[error("swapping {alpha_asset:?} for {beta_asset:?} from {alpha_ledger:?} to {beta_ledger:?} is not supported")]
pub struct UnsupportedSwap {
    alpha_asset: HttpAsset,
    beta_asset: HttpAsset,
    alpha_ledger: HttpLedger,
    beta_ledger: HttpLedger,
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
    alpha_ledger_refund_identity: Option<identity::Ethereum>,
    beta_ledger_redeem_identity: Option<identity::Ethereum>,
}

impl HttpIdentities {
    fn into_bitcoin_ethereum_identities(
        self,
        secret_source: &dyn DeriveIdentities,
    ) -> anyhow::Result<Identities<identity::Bitcoin, identity::Ethereum>> {
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

        let alpha_ledger_refund_identity = identity::Bitcoin::from_secret_key(
            &*crate::SECP,
            &secret_source.derive_refund_identity(),
        );

        Ok(Identities {
            alpha_ledger_refund_identity,
            beta_ledger_redeem_identity,
        })
    }

    fn into_ethereum_bitcoin_identities(
        self,
        secret_source: &dyn DeriveIdentities,
    ) -> anyhow::Result<Identities<identity::Ethereum, identity::Bitcoin>> {
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

        let beta_ledger_redeem_identity = identity::Bitcoin::from_secret_key(
            &*crate::SECP,
            &secret_source.derive_redeem_identity(),
        );

        Ok(Identities {
            alpha_ledger_refund_identity,
            beta_ledger_redeem_identity,
        })
    }
}

#[derive(Debug, Clone)]
struct Identities<AI, BI> {
    pub alpha_ledger_refund_identity: AI,
    pub beta_ledger_redeem_identity: BI,
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
    source: <identity::Ethereum as FromStr>::Err,
}

#[derive(strum_macros::Display, Debug, Clone, Copy)]
#[strum(serialize_all = "snake_case")]
pub enum IdentityKind {
    AlphaLedgerRefundIdentity,
    BetaLedgerRedeemIdentity,
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
    use crate::{
        network::DialInformation,
        swap_protocols::ledger::{self, ethereum::ChainId},
    };
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
                    "chain_id": 17
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
                    "chain_id": 17
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

        assert_that(&body)
            .is_ok()
            .map(|b| &b.beta_ledger)
            .is_equal_to(&HttpLedger::Ethereum(ledger::Ethereum {
                chain_id: ChainId::from(17),
            }));
    }
}
