pub mod index;
pub mod peers;
pub mod post;
pub mod rfc003;

use crate::{
    asset,
    db::Load,
    ethereum::{Bytes, ChainId},
    http_api::{action::ActionResponseBody, problem, route_factory, Http, Swap},
    swap_protocols::{
        actions::{
            ethereum,
            lnd::{self, Chain},
        },
        halight::{self, Settled},
        herc20, DeployAction, Facade, FundAction, InitAction, LocalSwapId, RedeemAction,
        RefundAction, Role,
    },
    timestamp::RelativeTime,
};
use ::comit::{Protocol, Secret, SecretHash};
use blockchain_contracts::ethereum::rfc003::{ether_htlc::EtherHtlc, Erc20Htlc};
use comit::{identity, Never, Timestamp};
use herc20::build_erc20_htlc;
use http_api_problem::HttpApiProblem;
use serde::Serialize;
use std::collections::HashMap;
use warp::{http, http::StatusCode, Rejection, Reply};

/// The lightning invoice expiry is used to tell the receiving lnd
/// until when should the payment of this invoice can be accepted.
///
/// If a payer tries to pay an expired invoice, lnd will automatically
/// reject the payment.
///
/// In the case of han-ether-halight, there are 3 expiries to take in
/// account:
/// 1. alpha expiry: The absolute time from when ether can be refunded to
/// Alice
/// 2. cltv or beta expiry: The relative time from when Bob can go on chain
/// to get his lightning bitcoin back. This is relative to when the
/// lightning htlc are sent to the blockchain as it uses
/// OP_CHECKSEQUENCEVERIFY.
/// 3. invoice expiry: The relative time from when Alice's lnd will not
/// accept a lightning payment from Bob. This is relative to when the hold
/// invoice is added by Alice to her lnd.
///
/// In terms of security, the beta expiry should expire before alpha expiry
/// with enough margin to ensure that Bob can refund his bitcoin (by going
/// on chain) before Alice can attempt to refund her ether.
///
/// So it should go:
/// cltv/beta expiry < min time to refund bitcoin < alpha expiry
///
/// The cltv expiry is relative so it means that once the values are agreed,
/// several actions needs to happen before we can now the actual (absolute)
/// beta expiry:
/// 1. Alice adds lnd invoice
/// 2. Bob send lnd payment
/// 3. Bob force closes the used lightning channel by broadcasting the
/// lightning htlcs.
/// 4. The lightning htlcs are mined in a block.
/// Once step 4 is done, then it is possible to know when bob can actually
/// refund his bitcoin.
///
/// Which means the following actions matter to keep the swap atomic:
/// 1. Alice and Bob agree on cltv and alpha expiry
///   > Alice control
/// 2. Alice adds lnd invoice
///   > Invoice expiry
/// 3. Bob sends lightning payment
///   > Bob control
/// 4. Bob force closes lightning channel
///   > Bitcoin blockchain
/// 5. Lightning htlcs are mined
///   > cltv expiry
/// 6. Lightning htlcs are expired
///   > Bob control/Immediate
/// 7. Bob sends Bitcoin refund transactions
///   > Bitcoin blockchain
/// 8. Bob's Bitcoin refund transactions are securely confirmed
///   > Alpha expiry
/// 9. Ether htlc is expired
///
/// If we only extract the waiting periods:
/// 0 -> Alice
///     -> invoice expiry
///         -> Bob
///             -> Bitcoin
///                 -> cltv expiry
///                     -> Bitcoin
///                         -> Alpha expiry
///
/// Note that the invoice expiry here protects Bob from locking its bitcoins
/// late in process, at a time where he tried to back out, it would not have
/// time to refund before Alice can redeem and refund.
///
/// We are currently setting the smallest expiry for Ethereum<>Bitcoin
/// onchain swaps to 12 hours but we do not recommend from Bob should
/// refrain to lock their asset. The invoice expiry value should be set to
/// this recommendation (that we currently do not provide).
///
/// Do not that Bob should not lock their funds immediately after Alice has
/// locked hers either. Bob should wait long enough to ensure that Alice's
/// asset cannot be sent to a different address by the way of a chain
/// re-org. According to various sources, it seems that 12 confirmations on
/// Ethereum (3min24s) is the equivalent of the 6 Bitcoin confirmations.
///
/// So Bob should probably wait at least 3 minutes after Alice locks her
/// Ether but not so long as to risk getting close to the absolute alpha
/// expiry.
///
/// Hence, 1 hour expiry seems to be a fair bet.
const INVOICE_EXPIRY_SECS: RelativeTime = RelativeTime::new(3600);

pub fn into_rejection(problem: HttpApiProblem) -> Rejection {
    warp::reject::custom(problem)
}

#[allow(clippy::needless_pass_by_value)]
pub async fn get_swap(swap_id: LocalSwapId, facade: Facade) -> Result<impl Reply, Rejection> {
    handle_get_swap(facade, swap_id)
        .await
        .map(|swap_resource| warp::reply::json(&swap_resource))
        .map_err(problem::from_anyhow)
        .map_err(into_rejection)
}

pub async fn handle_get_swap(
    facade: Facade,
    swap_id: LocalSwapId,
) -> anyhow::Result<siren::Entity> {
    match facade.load(swap_id).await? {
        Swap {
            alpha: Protocol::Herc20,
            beta: Protocol::Halight,
            role: Role::Alice,
        } => {
            let swap = facade.get_alice_herc20_halight_swap(swap_id).await?;
            make_swap_entity(swap_id, swap)
        }
        Swap {
            alpha: Protocol::Herc20,
            beta: Protocol::Halight,
            role: Role::Bob,
        } => {
            let swap = facade.get_bob_herc20_halight_swap(swap_id).await?;
            make_swap_entity(swap_id, swap)
        }
        _ => unimplemented!("only Herc20-Halight is supported"),
    }
}

fn make_swap_entity<S>(swap_id: LocalSwapId, swap: S) -> anyhow::Result<siren::Entity>
where
    S: GetRole
        + GetAlphaParams
        + GetBetaParams
        + GetAlphaEvents
        + GetBetaEvents
        + DeployAction
        + InitAction
        + FundAction
        + RedeemAction
        + RefundAction
        + Clone,
{
    let role = swap.get_role();
    let swap_resource = SwapResource { role: Http(role) };

    let mut entity = siren::Entity::default()
        .with_class_member("swap")
        .with_properties(swap_resource)
        .map_err(|e| {
            tracing::error!("failed to set properties of entity: {:?}", e);
            HttpApiProblem::with_title_and_type_from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?
        .with_link(siren::NavigationalLink::new(
            &["self"],
            route_factory::swap_path(swap_id),
        ));

    let alpha_params = swap.get_alpha_params();
    let alpha_params_sub = siren::SubEntity::from_entity(
        siren::Entity::default()
            .with_class_member("parameters")
            .with_properties(alpha_params)
            .map_err(|e| {
                tracing::error!("failed to set properties of entity: {:?}", e);
                HttpApiProblem::with_title_and_type_from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?,
        &["alpha"],
    );
    entity.push_sub_entity(alpha_params_sub);

    let beta_params = swap.get_beta_params();
    let beta_params_sub = siren::SubEntity::from_entity(
        siren::Entity::default()
            .with_class_member("parameters")
            .with_properties(beta_params)
            .map_err(|e| {
                tracing::error!("failed to set properties of entity: {:?}", e);
                HttpApiProblem::with_title_and_type_from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?,
        &["beta"],
    );
    entity.push_sub_entity(beta_params_sub);

    match (swap.get_alpha_events(), swap.get_beta_events()) {
        (Some(alpha_tx), Some(beta_tx)) => {
            let alpha_state_sub = siren::SubEntity::from_entity(
                siren::Entity::default()
                    .with_class_member("state")
                    .with_properties(alpha_tx)
                    .map_err(|e| {
                        tracing::error!("failed to set properties of entity: {:?}", e);
                        HttpApiProblem::with_title_and_type_from_status(
                            StatusCode::INTERNAL_SERVER_ERROR,
                        )
                    })?,
                &["alpha"],
            );
            entity.push_sub_entity(alpha_state_sub);

            let beta_state_sub = siren::SubEntity::from_entity(
                siren::Entity::default()
                    .with_class_member("state")
                    .with_properties(beta_tx)
                    .map_err(|e| {
                        tracing::error!("failed to set properties of entity: {:?}", e);
                        HttpApiProblem::with_title_and_type_from_status(
                            StatusCode::INTERNAL_SERVER_ERROR,
                        )
                    })?,
                &["beta"],
            );
            entity.push_sub_entity(beta_state_sub);

            let maybe_action_names = vec![
                swap.init_action().map(|_| "init"),
                swap.deploy_action().map(|_| "deploy"),
                swap.fund_action().map(|_| "fund"),
                swap.redeem_action().map(|_| "redeem"),
                swap.refund_action().map(|_| "refund"),
            ];

            Ok(maybe_action_names
                .into_iter()
                .filter_map(|action| action.ok())
                .fold(entity, |acc, action_name| {
                    let siren_action = make_siren_action(swap_id, action_name);
                    acc.with_action(siren_action)
                }))
        }
        _ => Ok(entity),
    }
}

fn make_siren_action(swap_id: LocalSwapId, action_name: &str) -> siren::Action {
    siren::Action {
        name: action_name.to_owned(),
        class: vec![],
        method: Some(http::Method::GET),
        href: format!("/swaps/{}/{}", swap_id, action_name),
        title: None,
        _type: None,
        fields: vec![],
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum SwapStatus {
    Created,
    InProgress,
    Swapped,
    NotSwapped,
}

#[derive(Debug, Serialize)]
struct SwapResource {
    pub role: Http<Role>,
}

trait GetAlphaEvents {
    fn get_alpha_events(&self) -> Option<LedgerEvents>;
}

trait GetBetaEvents {
    fn get_beta_events(&self) -> Option<LedgerEvents>;
}

trait GetRole {
    fn get_role(&self) -> Role;
}

trait GetAlphaParams {
    type Output: Serialize;
    fn get_alpha_params(&self) -> Self::Output;
}

trait GetBetaParams {
    type Output: Serialize;
    fn get_beta_params(&self) -> Self::Output;
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug)]
pub enum AliceHerc20HalightBitcoinSwap {
    Created {
        herc20_asset: asset::Erc20,
        halight_asset: asset::Bitcoin,
    },
    Finalized {
        herc20_asset: asset::Erc20,
        herc20_refund_identity: identity::Ethereum,
        herc20_redeem_identity: identity::Ethereum,
        herc20_expiry: Timestamp,
        herc20_state: herc20::State,
        halight_asset: asset::Bitcoin,
        halight_refund_identity: identity::Lightning,
        halight_redeem_identity: identity::Lightning,
        cltv_expiry: RelativeTime,
        halight_state: halight::State,
        secret: Secret,
    },
}

impl From<AliceHerc20HalightBitcoinSwap> for Herc20 {
    fn from(from: AliceHerc20HalightBitcoinSwap) -> Self {
        match from {
            AliceHerc20HalightBitcoinSwap::Created { herc20_asset, .. }
            | AliceHerc20HalightBitcoinSwap::Finalized { herc20_asset, .. } => Self {
                protocol: "herc20".to_owned(),
                quantity: herc20_asset.quantity.to_wei_dec(),
                token_contract: herc20_asset.token_contract.to_string(),
            },
        }
    }
}

impl From<AliceHerc20HalightBitcoinSwap> for Halight {
    fn from(from: AliceHerc20HalightBitcoinSwap) -> Self {
        match from {
            AliceHerc20HalightBitcoinSwap::Created { halight_asset, .. }
            | AliceHerc20HalightBitcoinSwap::Finalized { halight_asset, .. } => Self {
                protocol: "halight".to_owned(),
                quantity: halight_asset.as_sat().to_string(),
            },
        }
    }
}

impl GetAlphaEvents for AliceHerc20HalightBitcoinSwap {
    fn get_alpha_events(&self) -> Option<LedgerEvents> {
        match self {
            AliceHerc20HalightBitcoinSwap::Created { .. } => None,
            AliceHerc20HalightBitcoinSwap::Finalized { herc20_state, .. } => {
                Some(From::<herc20::State>::from(herc20_state.clone()))
            }
        }
    }
}

impl GetBetaEvents for AliceHerc20HalightBitcoinSwap {
    fn get_beta_events(&self) -> Option<LedgerEvents> {
        match self {
            AliceHerc20HalightBitcoinSwap::Created { .. } => None,
            AliceHerc20HalightBitcoinSwap::Finalized { halight_state, .. } => {
                Some(From::<halight::State>::from(*halight_state))
            }
        }
    }
}

impl GetRole for AliceHerc20HalightBitcoinSwap {
    fn get_role(&self) -> Role {
        Role::Alice
    }
}

impl GetAlphaParams for AliceHerc20HalightBitcoinSwap {
    type Output = Herc20;
    fn get_alpha_params(&self) -> Self::Output {
        self.clone().into()
    }
}

impl GetBetaParams for AliceHerc20HalightBitcoinSwap {
    type Output = Halight;
    fn get_beta_params(&self) -> Self::Output {
        self.clone().into()
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug)]
pub enum BobHerc20HalightBitcoinSwap {
    Created {
        herc20_asset: asset::Erc20,
        halight_asset: asset::Bitcoin,
    },
    Finalized {
        herc20_asset: asset::Erc20,
        herc20_refund_identity: identity::Ethereum,
        herc20_redeem_identity: identity::Ethereum,
        herc20_expiry: Timestamp,
        herc20_state: herc20::State,
        halight_asset: asset::Bitcoin,
        halight_refund_identity: identity::Lightning,
        halight_redeem_identity: identity::Lightning,
        cltv_expiry: RelativeTime,
        halight_state: halight::State,
        secret_hash: SecretHash,
    },
}

impl From<BobHerc20HalightBitcoinSwap> for Herc20 {
    fn from(from: BobHerc20HalightBitcoinSwap) -> Self {
        match from {
            BobHerc20HalightBitcoinSwap::Created { herc20_asset, .. }
            | BobHerc20HalightBitcoinSwap::Finalized { herc20_asset, .. } => Self {
                protocol: "herc20".to_owned(),
                quantity: herc20_asset.quantity.to_wei_dec(),
                token_contract: herc20_asset.token_contract.to_string(),
            },
        }
    }
}

impl From<BobHerc20HalightBitcoinSwap> for Halight {
    fn from(from: BobHerc20HalightBitcoinSwap) -> Self {
        match from {
            BobHerc20HalightBitcoinSwap::Created { halight_asset, .. }
            | BobHerc20HalightBitcoinSwap::Finalized { halight_asset, .. } => Self {
                protocol: "halight".to_owned(),
                quantity: halight_asset.as_sat().to_string(),
            },
        }
    }
}

impl GetAlphaEvents for BobHerc20HalightBitcoinSwap {
    fn get_alpha_events(&self) -> Option<LedgerEvents> {
        match self {
            BobHerc20HalightBitcoinSwap::Created { .. } => None,
            BobHerc20HalightBitcoinSwap::Finalized { herc20_state, .. } => {
                Some(From::<herc20::State>::from(herc20_state.clone()))
            }
        }
    }
}

impl GetBetaEvents for BobHerc20HalightBitcoinSwap {
    fn get_beta_events(&self) -> Option<LedgerEvents> {
        match self {
            BobHerc20HalightBitcoinSwap::Created { .. } => None,
            BobHerc20HalightBitcoinSwap::Finalized { halight_state, .. } => {
                Some(From::<halight::State>::from(*halight_state))
            }
        }
    }
}

impl GetRole for BobHerc20HalightBitcoinSwap {
    fn get_role(&self) -> Role {
        Role::Bob
    }
}

impl GetAlphaParams for BobHerc20HalightBitcoinSwap {
    type Output = Herc20;
    fn get_alpha_params(&self) -> Self::Output {
        self.clone().into()
    }
}

impl GetBetaParams for BobHerc20HalightBitcoinSwap {
    type Output = Halight;
    fn get_beta_params(&self) -> Self::Output {
        self.clone().into()
    }
}

#[derive(Debug, Serialize)]
struct Herc20 {
    pub protocol: String,
    pub quantity: String, // In Wei.
    pub token_contract: String,
}

#[derive(Debug, Serialize)]
struct Halight {
    pub protocol: String,
    pub quantity: String, // In Satoshi.
}

#[derive(Debug, Serialize)]
struct LedgerEvents {
    /// Keys are on of: "init", "deploy", "fund", "redeem", "refund".
    /// Values are transactions.
    transactions: HashMap<String, String>,
    status: EscrowStatus,
}

impl LedgerEvents {
    fn new(status: EscrowStatus) -> Self {
        Self {
            transactions: HashMap::new(), /* if we want transaction here, we should save the
                                           * events to the DB */
            status,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum EscrowStatus {
    None,
    Initialized,
    Deployed,
    Funded,
    Redeemed,
    Refunded,
    IncorrectlyFunded,
}

impl From<herc20::State> for LedgerEvents {
    fn from(state: herc20::State) -> Self {
        match state {
            herc20::State::None => LedgerEvents::new(EscrowStatus::None),
            herc20::State::Deployed { .. } => LedgerEvents::new(EscrowStatus::Deployed),
            herc20::State::Funded { .. } => LedgerEvents::new(EscrowStatus::Funded),
            herc20::State::IncorrectlyFunded { .. } => {
                LedgerEvents::new(EscrowStatus::IncorrectlyFunded)
            }
            herc20::State::Redeemed { .. } => LedgerEvents::new(EscrowStatus::Redeemed),
            herc20::State::Refunded { .. } => LedgerEvents::new(EscrowStatus::Refunded),
        }
    }
}

impl From<halight::State> for LedgerEvents {
    fn from(state: halight::State) -> Self {
        match state {
            halight::State::None => LedgerEvents {
                transactions: HashMap::new(),
                status: EscrowStatus::None,
            },
            halight::State::Opened(_) => LedgerEvents {
                transactions: HashMap::new(),
                status: EscrowStatus::Initialized,
            },
            halight::State::Accepted(_) => LedgerEvents {
                transactions: HashMap::new(),
                status: EscrowStatus::Funded,
            },
            halight::State::Settled(_) => LedgerEvents {
                transactions: HashMap::new(),
                status: EscrowStatus::Redeemed,
            },
            halight::State::Cancelled(_) => LedgerEvents {
                transactions: HashMap::new(),
                status: EscrowStatus::Refunded,
            },
        }
    }
}

impl InitAction for AliceHerc20HalightBitcoinSwap {
    type Output = lnd::AddHoldInvoice;

    fn init_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            AliceHerc20HalightBitcoinSwap::Finalized {
                halight_state: halight::State::None,
                halight_asset,
                halight_redeem_identity,
                cltv_expiry,
                secret,
                ..
            } => {
                let amount = *halight_asset;
                let secret_hash = SecretHash::new(*secret);
                let expiry = INVOICE_EXPIRY_SECS;
                let cltv_expiry = *cltv_expiry;
                let chain = Chain::Bitcoin;
                let network = bitcoin::Network::Regtest;
                let self_public_key = *halight_redeem_identity;

                Ok(lnd::AddHoldInvoice {
                    amount,
                    secret_hash,
                    expiry,
                    cltv_expiry,
                    chain,
                    network,
                    self_public_key,
                })
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl DeployAction for AliceHerc20HalightBitcoinSwap {
    type Output = ethereum::DeployContract;

    fn deploy_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            AliceHerc20HalightBitcoinSwap::Finalized {
                halight_state: halight::State::Opened(_),
                herc20_asset,
                herc20_refund_identity,
                herc20_redeem_identity,
                herc20_expiry,
                secret,
                ..
            } => {
                let htlc = build_erc20_htlc(
                    herc20_asset.clone(),
                    *herc20_redeem_identity,
                    *herc20_refund_identity,
                    *herc20_expiry,
                    SecretHash::new(*secret),
                );
                let gas_limit = Erc20Htlc::deploy_tx_gas_limit();
                let chain_id = ChainId::regtest();

                Ok(ethereum::DeployContract {
                    data: htlc.into(),
                    amount: asset::Ether::zero(),
                    gas_limit,
                    chain_id,
                })
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl FundAction for AliceHerc20HalightBitcoinSwap {
    type Output = ethereum::CallContract;

    fn fund_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            AliceHerc20HalightBitcoinSwap::Finalized {
                herc20_state: herc20::State::Deployed { htlc_location, .. },
                halight_state: halight::State::Opened(_),
                herc20_asset,
                ..
            } => {
                let herc20_asset = herc20_asset.clone();
                let to = herc20_asset.token_contract;
                let htlc_address = blockchain_contracts::ethereum::Address((*htlc_location).into());
                let data = Erc20Htlc::transfer_erc20_tx_payload(
                    herc20_asset.quantity.into(),
                    htlc_address,
                );
                let data = Some(Bytes(data));

                let gas_limit = Erc20Htlc::fund_tx_gas_limit();
                let chain_id = ChainId::regtest();
                let min_block_timestamp = None;

                Ok(ethereum::CallContract {
                    to,
                    data,
                    gas_limit,
                    chain_id,
                    min_block_timestamp,
                })
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl RedeemAction for AliceHerc20HalightBitcoinSwap {
    type Output = lnd::SettleInvoice;

    fn redeem_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            AliceHerc20HalightBitcoinSwap::Finalized {
                halight_state: halight::State::Accepted(_),
                halight_redeem_identity,
                secret,
                ..
            } => {
                let secret = *secret;
                let chain = Chain::Bitcoin;
                let network = bitcoin::Network::Regtest;
                let self_public_key = *halight_redeem_identity;

                Ok(lnd::SettleInvoice {
                    secret,
                    chain,
                    network,
                    self_public_key,
                })
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl RefundAction for AliceHerc20HalightBitcoinSwap {
    type Output = ethereum::CallContract;

    fn refund_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            AliceHerc20HalightBitcoinSwap::Finalized {
                herc20_state: herc20::State::Funded { htlc_location, .. },
                halight_state: halight::State::Accepted(_),
                herc20_expiry,
                ..
            } => {
                let to = *htlc_location;
                let data = None;
                let gas_limit = EtherHtlc::refund_tx_gas_limit();
                let chain_id = ChainId::regtest();
                let min_block_timestamp = Some(*herc20_expiry);

                Ok(ethereum::CallContract {
                    to,
                    data,
                    gas_limit,
                    chain_id,
                    min_block_timestamp,
                })
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl InitAction for BobHerc20HalightBitcoinSwap {
    type Output = Never;
    fn init_action(&self) -> anyhow::Result<Self::Output> {
        anyhow::bail!(ActionNotFound)
    }
}

impl DeployAction for BobHerc20HalightBitcoinSwap {
    type Output = Never;
    fn deploy_action(&self) -> anyhow::Result<Self::Output> {
        anyhow::bail!(ActionNotFound)
    }
}

impl RefundAction for BobHerc20HalightBitcoinSwap {
    type Output = Never;
    fn refund_action(&self) -> anyhow::Result<Self::Output> {
        anyhow::bail!(ActionNotFound)
    }
}

impl FundAction for BobHerc20HalightBitcoinSwap {
    type Output = lnd::SendPayment;

    fn fund_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            BobHerc20HalightBitcoinSwap::Finalized {
                herc20_state: herc20::State::Funded { .. },
                halight_state: halight::State::Opened(_),
                halight_asset,
                halight_refund_identity,
                halight_redeem_identity,
                cltv_expiry,
                secret_hash,
                ..
            } => {
                let to_public_key = *halight_redeem_identity;
                let amount = *halight_asset;
                let secret_hash = *secret_hash;
                let final_cltv_delta = *cltv_expiry;
                let chain = Chain::Bitcoin;
                let network = bitcoin::Network::Regtest;
                let self_public_key = *halight_refund_identity;

                Ok(lnd::SendPayment {
                    to_public_key,
                    amount,
                    secret_hash,
                    final_cltv_delta,
                    chain,
                    network,
                    self_public_key,
                })
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl RedeemAction for BobHerc20HalightBitcoinSwap {
    type Output = ethereum::CallContract;

    fn redeem_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            BobHerc20HalightBitcoinSwap::Finalized {
                herc20_state: herc20::State::Funded { htlc_location, .. },
                halight_state: halight::State::Settled(Settled { secret }),
                ..
            } => {
                let to = *htlc_location;
                let data = Some(Bytes::from(secret.into_raw_secret().to_vec()));
                let gas_limit = EtherHtlc::redeem_tx_gas_limit();
                let chain_id = ChainId::regtest();
                let min_block_timestamp = None;

                Ok(ethereum::CallContract {
                    to,
                    data,
                    gas_limit,
                    chain_id,
                    min_block_timestamp,
                })
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
pub async fn action_init(swap_id: LocalSwapId, facade: Facade) -> Result<impl Reply, Rejection> {
    handle_action_init(swap_id, facade)
        .await
        .map(|body| warp::reply::json(&body))
        .map_err(problem::from_anyhow)
        .map_err(into_rejection)
}

#[allow(clippy::unit_arg, clippy::let_unit_value, clippy::cognitive_complexity)]
async fn handle_action_init(id: LocalSwapId, facade: Facade) -> anyhow::Result<ActionResponseBody> {
    let action = facade
        .get_alice_herc20_halight_swap(id)
        .await?
        .init_action()?;
    Ok(ActionResponseBody::from(action))
}

#[allow(clippy::needless_pass_by_value)]
pub async fn action_deploy(swap_id: LocalSwapId, facade: Facade) -> Result<impl Reply, Rejection> {
    handle_action_deploy(swap_id, facade)
        .await
        .map(|body| warp::reply::json(&body))
        .map_err(problem::from_anyhow)
        .map_err(into_rejection)
}

#[allow(clippy::unit_arg, clippy::let_unit_value, clippy::cognitive_complexity)]
async fn handle_action_deploy(
    id: LocalSwapId,
    facade: Facade,
) -> anyhow::Result<ActionResponseBody> {
    let action = facade
        .get_alice_herc20_halight_swap(id)
        .await?
        .deploy_action()?;
    Ok(ActionResponseBody::from(action))
}

#[allow(clippy::needless_pass_by_value)]
pub async fn action_fund(swap_id: LocalSwapId, facade: Facade) -> Result<impl Reply, Rejection> {
    handle_action_fund(swap_id, facade)
        .await
        .map(|body| warp::reply::json(&body))
        .map_err(problem::from_anyhow)
        .map_err(into_rejection)
}

#[allow(clippy::unit_arg, clippy::let_unit_value, clippy::cognitive_complexity)]
async fn handle_action_fund(id: LocalSwapId, facade: Facade) -> anyhow::Result<ActionResponseBody> {
    match facade.load(id).await? {
        Role::Alice => {
            let action = facade
                .get_alice_herc20_halight_swap(id)
                .await?
                .fund_action()?;
            Ok(ActionResponseBody::from(action))
        }
        Role::Bob => {
            let action = facade
                .get_bob_herc20_halight_swap(id)
                .await?
                .fund_action()?;
            Ok(ActionResponseBody::from(action))
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
pub async fn action_redeem(swap_id: LocalSwapId, facade: Facade) -> Result<impl Reply, Rejection> {
    handle_action_redeem(swap_id, facade)
        .await
        .map(|body| warp::reply::json(&body))
        .map_err(problem::from_anyhow)
        .map_err(into_rejection)
}

#[allow(clippy::unit_arg, clippy::let_unit_value, clippy::cognitive_complexity)]
async fn handle_action_redeem(
    id: LocalSwapId,
    facade: Facade,
) -> anyhow::Result<ActionResponseBody> {
    match facade.load(id).await? {
        Role::Alice => {
            let action = facade
                .get_alice_herc20_halight_swap(id)
                .await?
                .redeem_action()?;
            Ok(ActionResponseBody::from(action))
        }
        Role::Bob => {
            let action = facade
                .get_bob_herc20_halight_swap(id)
                .await?
                .redeem_action()?;
            Ok(ActionResponseBody::from(action))
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
pub async fn action_refund(swap_id: LocalSwapId, facade: Facade) -> Result<impl Reply, Rejection> {
    handle_action_refund(swap_id, facade)
        .await
        .map(|body| warp::reply::json(&body))
        .map_err(problem::from_anyhow)
        .map_err(into_rejection)
}

#[allow(clippy::unit_arg, clippy::let_unit_value, clippy::cognitive_complexity)]
async fn handle_action_refund(
    id: LocalSwapId,
    facade: Facade,
) -> anyhow::Result<ActionResponseBody> {
    match facade.load(id).await? {
        Role::Alice => {
            let action = facade
                .get_alice_herc20_halight_swap(id)
                .await?
                .refund_action()?;
            Ok(ActionResponseBody::from(action))
        }
        Role::Bob => Err(ActionNotFound.into()),
    }
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("action not found")]
pub struct ActionNotFound;
