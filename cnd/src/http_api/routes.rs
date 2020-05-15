pub mod index;
pub mod peers;
pub mod post;
pub mod rfc003;

use crate::{
    asset,
    db::Load,
    ethereum::{Bytes, ChainId},
    http_api::{action::ActionResponseBody, problem, route_factory, DisplaySwap, Http},
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
use ::comit::{Secret, SecretHash};
use blockchain_contracts::ethereum::rfc003::{ether_htlc::EtherHtlc, Erc20Htlc};
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
pub async fn get_halight_swap(
    swap_id: LocalSwapId,
    facade: Facade,
) -> Result<impl Reply, Rejection> {
    handle_get_halight_swap(facade, swap_id)
        .await
        .map(|swap_resource| warp::reply::json(&swap_resource))
        .map_err(problem::from_anyhow)
        .map_err(into_rejection)
}

pub async fn handle_get_halight_swap(
    facade: Facade,
    swap_id: LocalSwapId,
) -> anyhow::Result<siren::Entity> {
    let swap: DisplaySwap<herc20::Asset, halight::Asset> = facade.load(swap_id).await?;

    match swap.role {
        Role::Alice => {
            let state = facade.get_alice_herc20_halight_swap(swap_id).await?;

            match state {
                Some(state) => {
                    // state.available_actions()
                    let maybe_action_names = vec![
                        state.init_action().map(|_| "init"),
                        state.deploy_action().map(|_| "deploy"),
                        state.fund_action().map(|_| "fund"),
                        state.redeem_action().map(|_| "redeem"),
                        state.refund_action().map(|_| "refund"),
                    ];
                    make_finalized_swap_herc20_halight_entity(
                        swap_id,
                        &swap,
                        state,
                        maybe_action_names,
                    )
                }
                None => make_swap_herc20_halight_entity(swap_id, &swap),
            }
        }
        Role::Bob => {
            let state = facade.get_bob_herc20_halight_swap(swap_id).await?;

            match state {
                Some(state) => {
                    let maybe_action_names = vec![
                        state.fund_action().map(|_| "fund"),
                        state.redeem_action().map(|_| "redeem"),
                    ];
                    make_finalized_swap_herc20_halight_entity(
                        swap_id,
                        &swap,
                        state,
                        maybe_action_names,
                    )
                }
                None => make_swap_herc20_halight_entity(swap_id, &swap),
            }
        }
    }
}

fn make_swap_herc20_halight_entity(
    swap_id: LocalSwapId,
    swap: &DisplaySwap<herc20::Asset, halight::Asset>,
) -> anyhow::Result<siren::Entity> {
    let role = swap.role;
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

    let alpha_params = HErc20 {
        protocol: "herc20".to_string(),
        quantity: swap.alpha_asset.0.quantity.to_wei_dec(),
        token_contract: swap.alpha_asset.0.token_contract.to_string(),
    };
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

    let beta_params = HalightBitcoin {
        protocol: "halight-bitcoin".to_string(),
        quantity: swap.beta_asset.0.as_sat().to_string(),
    };
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

    Ok(entity)
}

fn make_finalized_swap_herc20_halight_entity<S>(
    swap_id: LocalSwapId,
    swap: &DisplaySwap<herc20::Asset, halight::Asset>,
    state: S,
    maybe_action_names: Vec<Option<&str>>,
) -> anyhow::Result<siren::Entity>
where
    S: GetSwapStatus
        + GetRole
        + QuantityWei
        + QuantitySatoshi
        + GetAlphaEvents
        + GetBetaEvents
        + Clone,
{
    let mut entity = make_swap_herc20_halight_entity(swap_id, swap)?;

    let alpha_tx = state.get_alpha_events();
    let alpha_state_sub = siren::SubEntity::from_entity(
        siren::Entity::default()
            .with_class_member("state")
            .with_properties(alpha_tx)
            .map_err(|e| {
                tracing::error!("failed to set properties of entity: {:?}", e);
                HttpApiProblem::with_title_and_type_from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?,
        &["alpha"],
    );
    entity.push_sub_entity(alpha_state_sub);

    let beta_tx = state.get_beta_events();
    let beta_state_sub = siren::SubEntity::from_entity(
        siren::Entity::default()
            .with_class_member("state")
            .with_properties(beta_tx)
            .map_err(|e| {
                tracing::error!("failed to set properties of entity: {:?}", e);
                HttpApiProblem::with_title_and_type_from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?,
        &["beta"],
    );
    entity.push_sub_entity(beta_state_sub);

    Ok(maybe_action_names
        .into_iter()
        .filter_map(|action| action)
        .fold(entity, |acc, action_name| {
            let siren_action = make_siren_action(swap_id, action_name);
            acc.with_action(siren_action)
        }))
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

trait GetSwapStatus {
    fn get_swap_status(&self) -> SwapStatus;
}

trait GetAlphaEvents {
    fn get_alpha_events(&self) -> LedgerEvents;
}

trait GetBetaEvents {
    fn get_beta_events(&self) -> LedgerEvents;
}

trait GetRole {
    fn get_role(&self) -> Role;
}

/// Return the ether swap quantity in wei.
trait QuantityWei {
    fn quantity_wei(&self) -> String;
}

/// Return the bitcoin swap quantity in satoshi.
trait QuantitySatoshi {
    fn quantity_satoshi(&self) -> String;
}

fn herc20_halight_swap_status(
    herc20_state: &herc20::State,
    halight_state: &halight::State,
) -> SwapStatus {
    match (herc20_state, halight_state) {
        (herc20::State::Redeemed { .. }, halight::State::Settled(_)) => SwapStatus::Swapped,
        (herc20::State::IncorrectlyFunded { .. }, _) => SwapStatus::NotSwapped,
        (herc20::State::Refunded { .. }, _) => SwapStatus::NotSwapped,
        (_, halight::State::Cancelled(_)) => SwapStatus::NotSwapped,
        _ => SwapStatus::InProgress,
    }
}

#[derive(Clone, Debug)]
pub struct AliceHerc20HalightBitcoinSwap {
    pub alpha_ledger_state: herc20::State,
    pub beta_ledger_state: halight::State,

    pub herc20_params: herc20::InProgressSwap,
    pub halight_params: halight::InProgressSwap,

    pub secret: Secret,
}

impl GetSwapStatus for AliceHerc20HalightBitcoinSwap {
    fn get_swap_status(&self) -> SwapStatus {
        herc20_halight_swap_status(&self.alpha_ledger_state, &self.beta_ledger_state)
    }
}

impl GetAlphaEvents for AliceHerc20HalightBitcoinSwap {
    fn get_alpha_events(&self) -> LedgerEvents {
        LedgerEvents::from(self.alpha_ledger_state.clone())
    }
}

impl GetBetaEvents for AliceHerc20HalightBitcoinSwap {
    fn get_beta_events(&self) -> LedgerEvents {
        LedgerEvents::from(self.beta_ledger_state)
    }
}

impl GetRole for AliceHerc20HalightBitcoinSwap {
    fn get_role(&self) -> Role {
        Role::Alice
    }
}

impl QuantityWei for AliceHerc20HalightBitcoinSwap {
    fn quantity_wei(&self) -> String {
        self.herc20_params.asset.quantity.to_wei_dec()
    }
}

impl QuantitySatoshi for AliceHerc20HalightBitcoinSwap {
    fn quantity_satoshi(&self) -> String {
        self.halight_params.asset.as_sat().to_string()
    }
}

#[derive(Clone, Debug)]
pub struct BobHerc20HalightBitcoinSwap {
    pub alpha_ledger_state: herc20::State,
    pub beta_ledger_state: halight::State,

    pub herc20_params: herc20::InProgressSwap,
    pub halight_params: halight::InProgressSwap,
    pub secret_hash: SecretHash,
}

impl GetSwapStatus for BobHerc20HalightBitcoinSwap {
    fn get_swap_status(&self) -> SwapStatus {
        herc20_halight_swap_status(&self.alpha_ledger_state, &self.beta_ledger_state)
    }
}

impl GetAlphaEvents for BobHerc20HalightBitcoinSwap {
    fn get_alpha_events(&self) -> LedgerEvents {
        LedgerEvents::from(self.alpha_ledger_state.clone())
    }
}

impl GetBetaEvents for BobHerc20HalightBitcoinSwap {
    fn get_beta_events(&self) -> LedgerEvents {
        LedgerEvents::from(self.beta_ledger_state)
    }
}

impl GetRole for BobHerc20HalightBitcoinSwap {
    fn get_role(&self) -> Role {
        Role::Bob
    }
}

impl QuantityWei for BobHerc20HalightBitcoinSwap {
    fn quantity_wei(&self) -> String {
        self.herc20_params.asset.quantity.to_wei_dec()
    }
}

impl QuantitySatoshi for BobHerc20HalightBitcoinSwap {
    fn quantity_satoshi(&self) -> String {
        self.halight_params.asset.as_sat().to_string()
    }
}

#[derive(Debug, Serialize)]
struct HErc20 {
    pub protocol: String,
    pub quantity: String,
    // In Wei.
    pub token_contract: String,
}

#[derive(Debug, Serialize)]
struct HalightBitcoin {
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

    fn init_action(&self) -> Option<Self::Output> {
        match self.beta_ledger_state {
            halight::State::None => {
                let amount = self.halight_params.asset;
                let secret_hash = SecretHash::new(self.secret);
                let expiry = INVOICE_EXPIRY_SECS;
                let cltv_expiry = self.halight_params.expiry;
                let chain = Chain::Bitcoin;
                let network = bitcoin::Network::Regtest;
                let self_public_key = self.halight_params.redeem_identity;

                Some(lnd::AddHoldInvoice {
                    amount,
                    secret_hash,
                    expiry,
                    cltv_expiry,
                    chain,
                    network,
                    self_public_key,
                })
            }
            _ => None,
        }
    }
}

impl DeployAction for AliceHerc20HalightBitcoinSwap {
    type Output = ethereum::DeployContract;

    fn deploy_action(&self) -> Option<Self::Output> {
        match self.beta_ledger_state {
            halight::State::Opened(_) => {
                let htlc = self
                    .herc20_params
                    .build_erc20_htlc(SecretHash::new(self.secret));
                let gas_limit = Erc20Htlc::deploy_tx_gas_limit();
                let chain_id = ChainId::regtest();

                Some(ethereum::DeployContract {
                    data: htlc.into(),
                    amount: asset::Ether::zero(),
                    gas_limit,
                    chain_id,
                })
            }
            _ => None,
        }
    }
}

impl FundAction for AliceHerc20HalightBitcoinSwap {
    type Output = ethereum::CallContract;

    fn fund_action(&self) -> Option<Self::Output> {
        match (&self.alpha_ledger_state, &self.beta_ledger_state) {
            (herc20::State::Deployed { htlc_location, .. }, halight::State::Opened(_)) => {
                let htlc_params = self.herc20_params.clone();
                let chain_id = ChainId::regtest();
                let gas_limit = Erc20Htlc::fund_tx_gas_limit();

                let htlc_address = blockchain_contracts::ethereum::Address((*htlc_location).into());

                let data = Erc20Htlc::transfer_erc20_tx_payload(
                    htlc_params.asset.quantity.into(),
                    htlc_address,
                );

                Some(ethereum::CallContract {
                    to: htlc_params.asset.token_contract,
                    data: Some(Bytes(data)),
                    gas_limit,
                    chain_id,
                    min_block_timestamp: None,
                })
            }
            _ => None,
        }
    }
}

impl RedeemAction for AliceHerc20HalightBitcoinSwap {
    type Output = lnd::SettleInvoice;

    fn redeem_action(&self) -> Option<Self::Output> {
        match self.beta_ledger_state {
            halight::State::Accepted(_) => {
                let secret = self.secret;
                let chain = Chain::Bitcoin;
                let network = bitcoin::Network::Regtest;
                let self_public_key = self.halight_params.redeem_identity;

                Some(lnd::SettleInvoice {
                    secret,
                    chain,
                    network,
                    self_public_key,
                })
            }
            _ => None,
        }
    }
}

impl RefundAction for AliceHerc20HalightBitcoinSwap {
    type Output = ethereum::CallContract;

    fn refund_action(&self) -> Option<Self::Output> {
        match (&self.alpha_ledger_state, &self.beta_ledger_state) {
            (herc20::State::Funded { htlc_location, .. }, halight::State::Accepted(_)) => {
                let to = *htlc_location;
                let data = None;
                let gas_limit = EtherHtlc::refund_tx_gas_limit();
                let chain_id = ChainId::regtest();
                let min_block_timestamp = Some(self.herc20_params.expiry);

                Some(ethereum::CallContract {
                    to,
                    data,
                    gas_limit,
                    chain_id,
                    min_block_timestamp,
                })
            }
            _ => None,
        }
    }
}

impl FundAction for BobHerc20HalightBitcoinSwap {
    type Output = lnd::SendPayment;

    fn fund_action(&self) -> Option<Self::Output> {
        match (&self.alpha_ledger_state, &self.beta_ledger_state) {
            (herc20::State::Funded { .. }, halight::State::Opened(_)) => {
                let to_public_key = self.halight_params.redeem_identity;
                let amount = self.halight_params.asset;
                let secret_hash = self.secret_hash;
                let final_cltv_delta = self.halight_params.expiry;
                let chain = Chain::Bitcoin;
                let network = bitcoin::Network::Regtest;
                let self_public_key = self.halight_params.refund_identity;

                Some(lnd::SendPayment {
                    to_public_key,
                    amount,
                    secret_hash,
                    final_cltv_delta,
                    chain,
                    network,
                    self_public_key,
                })
            }
            _ => None,
        }
    }
}

impl RedeemAction for BobHerc20HalightBitcoinSwap {
    type Output = ethereum::CallContract;

    fn redeem_action(&self) -> Option<Self::Output> {
        match (&self.alpha_ledger_state, &self.beta_ledger_state) {
            (
                herc20::State::Funded { htlc_location, .. },
                halight::State::Settled(Settled { secret }),
            ) => {
                let to = *htlc_location;
                let data = Some(Bytes::from(secret.into_raw_secret().to_vec()));
                let gas_limit = EtherHtlc::redeem_tx_gas_limit();
                let chain_id = ChainId::regtest();
                let min_block_timestamp = None;

                Some(ethereum::CallContract {
                    to,
                    data,
                    gas_limit,
                    chain_id,
                    min_block_timestamp,
                })
            }
            _ => None,
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
    let role = Load::<Role>::load(&facade, id).await?;

    let maybe_response = match role {
        Role::Alice => {
            let state = facade.get_alice_herc20_halight_swap(id).await?;

            state
                .map(|state| state.init_action().map(ActionResponseBody::from))
                .flatten()
        }
        Role::Bob => None,
    };

    let response = maybe_response.ok_or(LndActionError::NotFound)?;

    Ok(response)
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
    let role = Load::<Role>::load(&facade, id).await?;

    let maybe_response = match role {
        Role::Alice => {
            let state = facade.get_alice_herc20_halight_swap(id).await?;

            state
                .map(|state| state.deploy_action().map(ActionResponseBody::from))
                .flatten()
        }
        Role::Bob => None,
    };

    let response = maybe_response.ok_or(LndActionError::NotFound)?;

    Ok(response)
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
    let role = Load::<Role>::load(&facade, id).await?;

    let maybe_response = match role {
        Role::Alice => {
            let state = facade.get_alice_herc20_halight_swap(id).await?;

            state
                .map(|state| state.fund_action().map(ActionResponseBody::from))
                .flatten()
        }
        Role::Bob => {
            let state = facade.get_bob_herc20_halight_swap(id).await?;

            state
                .map(|state| state.fund_action().map(ActionResponseBody::from))
                .flatten()
        }
    };

    let response = maybe_response.ok_or(LndActionError::NotFound)?;

    Ok(response)
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
    let role = Load::<Role>::load(&facade, id).await?;

    let maybe_response = match role {
        Role::Alice => {
            let state = facade.get_alice_herc20_halight_swap(id).await?;

            state
                .map(|state| state.redeem_action().map(ActionResponseBody::from))
                .flatten()
        }
        Role::Bob => {
            let state = facade.get_bob_herc20_halight_swap(id).await?;

            state
                .map(|state| state.redeem_action().map(ActionResponseBody::from))
                .flatten()
        }
    };

    let response = maybe_response.ok_or(LndActionError::NotFound)?;

    Ok(response)
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
    let role = Load::<Role>::load(&facade, id).await?;

    let maybe_response = match role {
        Role::Alice => {
            let state = facade.get_alice_herc20_halight_swap(id).await?;

            state
                .map(|state| state.refund_action().map(ActionResponseBody::from))
                .flatten()
        }
        Role::Bob => None,
    };

    let response = maybe_response.ok_or(LndActionError::NotFound)?;

    Ok(response)
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum LndActionError {
    #[error("action not found")]
    NotFound,
}
