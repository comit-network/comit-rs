pub mod index;
pub mod peers;
pub mod rfc003;

use crate::{
    asset,
    ethereum::Bytes,
    htlc_location,
    http_api::{action::ActionResponseBody, problem, route_factory, Http},
    network::comit_ln,
    swap_protocols::{
        actions::{
            ethereum,
            lnd::{self, Chain},
        },
        halight::{self, Settled},
        ledger::ethereum::ChainId,
        rfc003::{ledger_state::HtlcState, LedgerState},
        state::Get,
        Facade, FundAction, HanEtherereumHalightBitcoinCreateSwapParams, InitAction, LocalSwapId,
        RedeemAction, RefundAction, Role,
    },
    transaction,
};
use blockchain_contracts::ethereum::rfc003::ether_htlc::EtherHtlc;
use http_api_problem::HttpApiProblem;
use serde::Serialize;
use std::collections::HashMap;
use warp::{http, http::StatusCode, Rejection, Reply};

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
    let alpha_ledger_state: Option<
        LedgerState<asset::Ether, htlc_location::Ethereum, transaction::Ethereum>,
    > = facade.alpha_ledger_states.get(&swap_id).await?;

    let beta_ledger_state = facade.beta_ledger_states.get(&swap_id).await?;

    let created_swap = facade.get_created_swap(swap_id).await;

    let finalized_swap = facade.get_finalized_swap(swap_id).await;

    let (alpha_ledger_state, beta_ledger_state, finalized_swap) = match (
        alpha_ledger_state,
        beta_ledger_state,
        finalized_swap,
        created_swap,
    ) {
        (Some(alpha_ledger_state), Some(beta_ledger_state), Some(finalized_swap), _) => {
            (alpha_ledger_state, beta_ledger_state, finalized_swap)
        }
        (_, _, _, Some(created_swap)) => {
            return make_created_swap_entity(swap_id, HanEthereumHalightBitcoinCreatedState {
                created_swap,
            })
        }
        _ => {
            let empty_swap = siren::Entity::default().with_class_member("swaps");

            tracing::debug!("returning empty siren document because states are not yet completed");

            return Ok(empty_swap);
        }
    };

    match finalized_swap.role {
        Role::Alice => {
            let state = AliceHanEthereumHalightBitcoinState {
                alpha_ledger_state,
                beta_ledger_state,
                finalized_swap,
            };

            let maybe_action_names = vec![
                state.init_action().map(|_| "init"),
                state.fund_action().map(|_| "fund"),
                state.redeem_action().map(|_| "redeem"),
                state.refund_action().map(|_| "refund"),
            ];
            make_finalized_swap_entity(swap_id, state, maybe_action_names)
        }
        Role::Bob => {
            let state = BobHanEthereumHalightBitcoinState {
                alpha_ledger_state,
                beta_ledger_state,
                finalized_swap,
            };

            // Bob cannot init and refund in this swap combination
            let maybe_action_names = vec![
                state.fund_action().map(|_| "fund"),
                state.redeem_action().map(|_| "redeem"),
            ];
            make_finalized_swap_entity(swap_id, state, maybe_action_names)
        }
    }
}

// TODO: Refactor with make_finalized_swap_entity
fn make_created_swap_entity<S>(swap_id: LocalSwapId, state: S) -> anyhow::Result<siren::Entity>
where
    S: GetSwapStatus + GetRole + QuantityWei + QuantitySatoshi,
{
    let role = state.get_role();
    let swap = SwapResource {
        status: state.get_swap_status(),
        role: Http(role),
    };

    let mut entity = siren::Entity::default()
        .with_class_member("swaps")
        .with_properties(swap)
        .map_err(|e| {
            tracing::error!("failed to set properties of entity: {:?}", e);
            HttpApiProblem::with_title_and_type_from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?
        .with_link(siren::NavigationalLink::new(
            &["self"],
            route_factory::swap_path(swap_id),
        ));

    let alpha_params = HanEthereum {
        protocol: "han-ethereum".to_string(),
        quantity: state.quantity_wei(),
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
        quantity: state.quantity_satoshi(),
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

fn make_finalized_swap_entity<S>(
    swap_id: LocalSwapId,
    state: S,
    maybe_action_names: Vec<Option<&str>>,
) -> anyhow::Result<siren::Entity>
where
    S: GetSwapStatus
        + GetRole
        + QuantityWei
        + QuantitySatoshi
        + GetAlphaTransaction
        + GetBetaTransaction,
{
    let role = state.get_role();
    let swap = SwapResource {
        status: state.get_swap_status(),
        role: Http(role),
    };

    let mut entity = siren::Entity::default()
        .with_class_member("swaps")
        .with_properties(swap)
        .map_err(|e| {
            tracing::error!("failed to set properties of entity: {:?}", e);
            HttpApiProblem::with_title_and_type_from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?
        .with_link(siren::NavigationalLink::new(
            &["self"],
            route_factory::swap_path(swap_id),
        ));

    let alpha_params = HanEthereum {
        protocol: "han-ethereum".to_string(),
        quantity: state.quantity_wei(),
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
        quantity: state.quantity_satoshi(),
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

    let alpha_tx = state.get_alpha_transaction();
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

    let beta_tx = state.get_beta_transaction();
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
    pub status: SwapStatus,
    pub role: Http<Role>,
}

trait GetSwapStatus {
    fn get_swap_status(&self) -> SwapStatus;
}

trait GetAlphaTransaction {
    fn get_alpha_transaction(&self) -> Transaction;
}

trait GetBetaTransaction {
    fn get_beta_transaction(&self) -> Transaction;
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

fn han_eth_halight_swap_status(
    ethereum_status: HtlcState,
    beta_ledger_state: &halight::State,
) -> SwapStatus {
    match (ethereum_status, beta_ledger_state) {
        (HtlcState::Redeemed, halight::State::Settled(_)) => SwapStatus::Swapped,
        (HtlcState::IncorrectlyFunded, _) => SwapStatus::NotSwapped,
        (HtlcState::Refunded, _) => SwapStatus::NotSwapped,
        (_, halight::State::Cancelled(_)) => SwapStatus::NotSwapped,
        _ => SwapStatus::InProgress,
    }
}

#[derive(Debug)]
pub struct HanEthereumHalightBitcoinCreatedState {
    pub created_swap: HanEtherereumHalightBitcoinCreateSwapParams,
}

impl GetSwapStatus for HanEthereumHalightBitcoinCreatedState {
    fn get_swap_status(&self) -> SwapStatus {
        SwapStatus::Created
    }
}

impl GetRole for HanEthereumHalightBitcoinCreatedState {
    fn get_role(&self) -> Role {
        self.created_swap.role
    }
}

impl QuantityWei for HanEthereumHalightBitcoinCreatedState {
    fn quantity_wei(&self) -> String {
        self.created_swap.ethereum_amount.to_wei_dec()
    }
}

impl QuantitySatoshi for HanEthereumHalightBitcoinCreatedState {
    fn quantity_satoshi(&self) -> String {
        self.created_swap.lightning_amount.as_sat().to_string()
    }
}

#[derive(Debug)]
pub struct AliceHanEthereumHalightBitcoinState {
    pub alpha_ledger_state:
        LedgerState<asset::Ether, htlc_location::Ethereum, transaction::Ethereum>,
    pub beta_ledger_state: halight::State,
    pub finalized_swap: comit_ln::FinalizedSwap,
}

impl GetSwapStatus for AliceHanEthereumHalightBitcoinState {
    fn get_swap_status(&self) -> SwapStatus {
        let ethereum_status = HtlcState::from(self.alpha_ledger_state.clone());
        han_eth_halight_swap_status(ethereum_status, &self.beta_ledger_state)
    }
}

impl GetAlphaTransaction for AliceHanEthereumHalightBitcoinState {
    fn get_alpha_transaction(&self) -> Transaction {
        Transaction::from(self.alpha_ledger_state.clone())
    }
}

impl GetBetaTransaction for AliceHanEthereumHalightBitcoinState {
    fn get_beta_transaction(&self) -> Transaction {
        Transaction::from(self.beta_ledger_state)
    }
}

impl GetRole for AliceHanEthereumHalightBitcoinState {
    fn get_role(&self) -> Role {
        Role::Alice
    }
}

impl QuantityWei for AliceHanEthereumHalightBitcoinState {
    fn quantity_wei(&self) -> String {
        self.finalized_swap.alpha_asset.to_wei_dec()
    }
}

impl QuantitySatoshi for AliceHanEthereumHalightBitcoinState {
    fn quantity_satoshi(&self) -> String {
        self.finalized_swap.beta_asset.as_sat().to_string()
    }
}

#[derive(Debug)]
pub struct BobHanEthereumHalightBitcoinState {
    pub alpha_ledger_state:
        LedgerState<asset::Ether, htlc_location::Ethereum, transaction::Ethereum>,
    pub beta_ledger_state: halight::State,
    pub finalized_swap: comit_ln::FinalizedSwap,
}

impl GetSwapStatus for BobHanEthereumHalightBitcoinState {
    fn get_swap_status(&self) -> SwapStatus {
        let ethereum_status = HtlcState::from(self.alpha_ledger_state.clone());
        han_eth_halight_swap_status(ethereum_status, &self.beta_ledger_state)
    }
}

impl GetAlphaTransaction for BobHanEthereumHalightBitcoinState {
    fn get_alpha_transaction(&self) -> Transaction {
        Transaction::from(self.alpha_ledger_state.clone())
    }
}

impl GetBetaTransaction for BobHanEthereumHalightBitcoinState {
    fn get_beta_transaction(&self) -> Transaction {
        Transaction::from(self.beta_ledger_state)
    }
}

impl GetRole for BobHanEthereumHalightBitcoinState {
    fn get_role(&self) -> Role {
        Role::Bob
    }
}

impl QuantityWei for BobHanEthereumHalightBitcoinState {
    fn quantity_wei(&self) -> String {
        self.finalized_swap.alpha_asset.to_wei_dec()
    }
}

impl QuantitySatoshi for BobHanEthereumHalightBitcoinState {
    fn quantity_satoshi(&self) -> String {
        self.finalized_swap.beta_asset.as_sat().to_string()
    }
}

#[derive(Debug, Serialize)]
struct HanEthereum {
    pub protocol: String,
    pub quantity: String, // In Wei.
}

#[derive(Debug, Serialize)]
struct HalightBitcoin {
    pub protocol: String,
    pub quantity: String, // In Satoshi.
}

#[derive(Debug, Serialize)]
struct Transaction {
    /// Keys are on of: "init", "deploy", "fund", "redeem", "refund".
    /// Values are transactions.
    transactions: HashMap<String, String>,
    status: EscrowStatus,
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

impl From<LedgerState<asset::Ether, htlc_location::Ethereum, transaction::Ethereum>>
    for Transaction
{
    fn from(
        state: LedgerState<asset::Ether, htlc_location::Ethereum, transaction::Ethereum>,
    ) -> Self {
        match state {
            LedgerState::NotDeployed => Transaction {
                transactions: HashMap::new(),
                status: EscrowStatus::None,
            },
            LedgerState::Deployed {
                deploy_transaction, ..
            } => {
                let mut transactions = HashMap::new();
                transactions.insert("deploy".to_string(), deploy_transaction.hash.to_string());

                Transaction {
                    transactions,
                    status: EscrowStatus::Deployed,
                }
            }
            LedgerState::Funded {
                deploy_transaction,
                fund_transaction,
                ..
            } => {
                let mut transactions = HashMap::new();
                transactions.insert("deploy".to_string(), deploy_transaction.hash.to_string());
                transactions.insert("fund".to_string(), fund_transaction.hash.to_string());
                Transaction {
                    transactions,
                    status: EscrowStatus::Funded,
                }
            }
            LedgerState::IncorrectlyFunded {
                deploy_transaction,
                fund_transaction,
                ..
            } => {
                let mut transactions = HashMap::new();
                transactions.insert("deploy".to_string(), deploy_transaction.hash.to_string());
                transactions.insert("fund".to_string(), fund_transaction.hash.to_string());
                Transaction {
                    transactions,
                    status: EscrowStatus::IncorrectlyFunded,
                }
            }
            LedgerState::Redeemed {
                deploy_transaction,
                fund_transaction,
                redeem_transaction,
                ..
            } => {
                let mut transactions = HashMap::new();
                transactions.insert("deploy".to_string(), deploy_transaction.hash.to_string());
                transactions.insert("fund".to_string(), fund_transaction.hash.to_string());
                transactions.insert("redeem".to_string(), redeem_transaction.hash.to_string());
                Transaction {
                    transactions,
                    status: EscrowStatus::Redeemed,
                }
            }
            LedgerState::Refunded {
                deploy_transaction,
                fund_transaction,
                refund_transaction,
                ..
            } => {
                let mut transactions = HashMap::new();
                transactions.insert("deploy".to_string(), deploy_transaction.hash.to_string());
                transactions.insert("fund".to_string(), fund_transaction.hash.to_string());
                transactions.insert("refund".to_string(), refund_transaction.hash.to_string());
                Transaction {
                    transactions,
                    status: EscrowStatus::Refunded,
                }
            }
        }
    }
}

impl From<halight::State> for Transaction {
    fn from(state: halight::State) -> Self {
        match state {
            halight::State::None => Transaction {
                transactions: HashMap::new(),
                status: EscrowStatus::None,
            },
            halight::State::Opened(_) => Transaction {
                transactions: HashMap::new(),
                status: EscrowStatus::Initialized,
            },
            halight::State::Accepted(_) => Transaction {
                transactions: HashMap::new(),
                status: EscrowStatus::Funded,
            },
            halight::State::Settled(_) => Transaction {
                transactions: HashMap::new(),
                status: EscrowStatus::Redeemed,
            },
            halight::State::Cancelled(_) => Transaction {
                transactions: HashMap::new(),
                status: EscrowStatus::Refunded,
            },
        }
    }
}

impl InitAction for AliceHanEthereumHalightBitcoinState {
    type Output = lnd::AddHoldInvoice;

    fn init_action(&self) -> Option<Self::Output> {
        match self.beta_ledger_state {
            halight::State::None => {
                let amount = self.finalized_swap.beta_asset;
                let secret_hash = self.finalized_swap.secret_hash;
                let expiry = 3600;
                let cltv_expiry = self.finalized_swap.beta_expiry.into();
                let chain = Chain::Bitcoin;
                let network = bitcoin::Network::Regtest;
                let self_public_key = self.finalized_swap.beta_ledger_redeem_identity;

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

impl FundAction for AliceHanEthereumHalightBitcoinState {
    type Output = ethereum::DeployContract;

    fn fund_action(&self) -> Option<Self::Output> {
        match self.beta_ledger_state {
            halight::State::Opened(_) => {
                let eth_htlc = self.finalized_swap.han_params();
                let data = eth_htlc.into();
                let amount = self.finalized_swap.alpha_asset.clone();
                let gas_limit = EtherHtlc::deploy_tx_gas_limit();
                let chain_id = ChainId::regtest();

                Some(ethereum::DeployContract {
                    data,
                    amount,
                    gas_limit,
                    chain_id,
                })
            }
            _ => None,
        }
    }
}

impl RedeemAction for AliceHanEthereumHalightBitcoinState {
    type Output = lnd::SettleInvoice;

    fn redeem_action(&self) -> Option<Self::Output> {
        match self.beta_ledger_state {
            halight::State::Accepted(_) => {
                let secret = self.finalized_swap.secret.unwrap(); // unwrap ok since only Alice calls this.
                let chain = Chain::Bitcoin;
                let network = bitcoin::Network::Regtest;
                let self_public_key = self.finalized_swap.beta_ledger_redeem_identity;

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

impl RefundAction for AliceHanEthereumHalightBitcoinState {
    type Output = ethereum::CallContract;

    fn refund_action(&self) -> Option<Self::Output> {
        match (&self.alpha_ledger_state, &self.beta_ledger_state) {
            (LedgerState::Funded { htlc_location, .. }, halight::State::Accepted(_)) => {
                let to = *htlc_location;
                let data = None;
                let gas_limit = EtherHtlc::refund_tx_gas_limit();
                let chain_id = ChainId::regtest();
                let min_block_timestamp = Some(self.finalized_swap.alpha_expiry);

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

impl FundAction for BobHanEthereumHalightBitcoinState {
    type Output = lnd::SendPayment;

    fn fund_action(&self) -> Option<Self::Output> {
        match (&self.alpha_ledger_state, &self.beta_ledger_state) {
            (LedgerState::Funded { .. }, halight::State::Opened(_)) => {
                let to_public_key = self.finalized_swap.beta_ledger_redeem_identity;
                let amount = self.finalized_swap.beta_asset;
                let secret_hash = self.finalized_swap.secret_hash;
                let final_cltv_delta = self.finalized_swap.beta_expiry.into();
                let chain = Chain::Bitcoin;
                let network = bitcoin::Network::Regtest;
                let self_public_key = self.finalized_swap.beta_ledger_refund_identity;

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

impl RedeemAction for BobHanEthereumHalightBitcoinState {
    type Output = ethereum::CallContract;

    fn redeem_action(&self) -> Option<Self::Output> {
        match (&self.alpha_ledger_state, &self.beta_ledger_state) {
            (
                LedgerState::Funded { htlc_location, .. },
                halight::State::Settled(Settled { secret }),
            ) => {
                let to = *htlc_location;
                let data = Some(Bytes::from(secret.into_raw_secret().to_vec()));
                let gas_limit = EtherHtlc::redeem_tx_gas_limit();
                let chain_id: ChainId = ChainId::regtest();
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
async fn handle_action_init(
    swap_id: LocalSwapId,
    facade: Facade,
) -> anyhow::Result<ActionResponseBody> {
    let alpha_ledger_state: LedgerState<
        asset::Ether,
        htlc_location::Ethereum,
        transaction::Ethereum,
    > = facade
        .alpha_ledger_states
        .get(&swap_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("alpha ledger state not found for {}", swap_id))?;

    let beta_ledger_state: halight::State = facade
        .beta_ledger_states
        .get(&swap_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("beta ledger state not found for {}", swap_id))?;

    let finalized_swap = facade
        .get_finalized_swap(swap_id)
        .await
        .ok_or_else(|| anyhow::anyhow!("swap with id {} not found", swap_id))?;

    let maybe_response = match finalized_swap.role {
        Role::Alice => {
            let state = AliceHanEthereumHalightBitcoinState {
                alpha_ledger_state,
                beta_ledger_state,
                finalized_swap,
            };

            state.init_action().map(ActionResponseBody::from)
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
async fn handle_action_fund(
    swap_id: LocalSwapId,
    facade: Facade,
) -> anyhow::Result<ActionResponseBody> {
    let alpha_ledger_state: LedgerState<
        asset::Ether,
        htlc_location::Ethereum,
        transaction::Ethereum,
    > = facade
        .alpha_ledger_states
        .get(&swap_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("alpha ledger state not found for {}", swap_id))?;

    let beta_ledger_state: halight::State = facade
        .beta_ledger_states
        .get(&swap_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("beta ledger state not found for {}", swap_id))?;

    let finalized_swap = facade
        .get_finalized_swap(swap_id)
        .await
        .ok_or_else(|| anyhow::anyhow!("swap with id {} not found", swap_id))?;

    let maybe_response = match finalized_swap.role {
        Role::Alice => {
            let state = AliceHanEthereumHalightBitcoinState {
                alpha_ledger_state,
                beta_ledger_state,
                finalized_swap,
            };

            state.fund_action().map(ActionResponseBody::from)
        }
        Role::Bob => {
            let state = BobHanEthereumHalightBitcoinState {
                alpha_ledger_state,
                beta_ledger_state,
                finalized_swap,
            };

            state.fund_action().map(ActionResponseBody::from)
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
    swap_id: LocalSwapId,
    facade: Facade,
) -> anyhow::Result<ActionResponseBody> {
    let alpha_ledger_state: LedgerState<
        asset::Ether,
        htlc_location::Ethereum,
        transaction::Ethereum,
    > = facade
        .alpha_ledger_states
        .get(&swap_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("alpha ledger state not found for {}", swap_id))?;

    let beta_ledger_state: halight::State = facade
        .beta_ledger_states
        .get(&swap_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("beta ledger state not found for {}", swap_id))?;

    let finalized_swap = facade
        .get_finalized_swap(swap_id)
        .await
        .ok_or_else(|| anyhow::anyhow!("swap with id {} not found", swap_id))?;

    let maybe_response = match finalized_swap.role {
        Role::Alice => {
            let state = AliceHanEthereumHalightBitcoinState {
                alpha_ledger_state,
                beta_ledger_state,
                finalized_swap,
            };

            state.redeem_action().map(ActionResponseBody::from)
        }
        Role::Bob => {
            let state = BobHanEthereumHalightBitcoinState {
                alpha_ledger_state,
                beta_ledger_state,
                finalized_swap,
            };

            state.redeem_action().map(ActionResponseBody::from)
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
    swap_id: LocalSwapId,
    facade: Facade,
) -> anyhow::Result<ActionResponseBody> {
    let alpha_ledger_state: LedgerState<
        asset::Ether,
        htlc_location::Ethereum,
        transaction::Ethereum,
    > = facade
        .alpha_ledger_states
        .get(&swap_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("alpha ledger state not found for {}", swap_id))?;

    let beta_ledger_state: halight::State = facade
        .beta_ledger_states
        .get(&swap_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("beta ledger state not found for {}", swap_id))?;

    let finalized_swap = facade
        .get_finalized_swap(swap_id)
        .await
        .ok_or_else(|| anyhow::anyhow!("swap with id {} not found", swap_id))?;

    let maybe_response = match finalized_swap.role {
        Role::Alice => {
            let state = AliceHanEthereumHalightBitcoinState {
                alpha_ledger_state,
                beta_ledger_state,
                finalized_swap,
            };

            state.refund_action().map(ActionResponseBody::from)
        }
        _ => None,
    };

    let response = maybe_response.ok_or(LndActionError::NotFound)?;

    Ok(response)
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum LndActionError {
    #[error("action not found")]
    NotFound,
}
