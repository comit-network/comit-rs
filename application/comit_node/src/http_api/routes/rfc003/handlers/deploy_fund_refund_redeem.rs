use crate::{
    http_api::{
        problem,
        routes::rfc003::action::{new_action_link, ListRequiredFields, ToSirenAction},
    },
    swap_protocols::{
        metadata_store::Metadata,
        rfc003::{alice, bitcoin, bob, ethereum, state_store::StateStore, Actions, Timestamp},
        MetadataStore, SwapId,
    },
};
use bitcoin_support::{self, serialize_hex, BitcoinQuantity, Network, Transaction};
use ethereum_support::{self, Erc20Token, EtherQuantity};
use http_api_problem::{HttpApiProblem, StatusCode};
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, sync::Arc};

pub fn handle_deploy_action<T: MetadataStore<SwapId>, S: StateStore>(
    metadata_store: &T,
    state_store: Arc<S>,
    id: &SwapId,
    query_params: &ActionExecutionParameters,
) -> Result<ActionResponseBody, HttpApiProblem> {
    let metadata = metadata_store
        .get(id)?
        .ok_or_else(problem::swap_not_found)?;

    with_swap_types!(
        &metadata,
        (|| {
            let state = state_store
                .get::<ROLE>(id.clone())?
                .ok_or_else(problem::state_store)?;
            log::trace!("Retrieved state for {}: {:?}", id, state);

            state
                .actions()
                .into_iter()
                .find_map(|action| match action {
                    Deploy(action) => Some(action.into_response_payload(query_params.clone())),
                    _ => None,
                })
                .unwrap_or_else(|| {
                    //                Err(problem::invalid_action(action_name))
                    unimplemented!()
                })
        })
    )
}
pub fn handle_fund_action<T: MetadataStore<SwapId>, S: StateStore>(
    metadata_store: &T,
    state_store: Arc<S>,
    id: &SwapId,
    query_params: &ActionExecutionParameters,
) -> Result<ActionResponseBody, HttpApiProblem> {
    let metadata = metadata_store
        .get(id)?
        .ok_or_else(problem::swap_not_found)?;

    with_swap_types!(
        &metadata,
        (|| {
            let state = state_store
                .get::<ROLE>(id.clone())?
                .ok_or_else(problem::state_store)?;
            log::trace!("Retrieved state for {}: {:?}", id, state);

            state
                .actions()
                .into_iter()
                .find_map(|action| match action {
                    Fund(action) => Some(action.into_response_payload(query_params.clone())),
                    _ => None,
                })
                .unwrap_or_else(|| {
                    //                Err(problem::invalid_action(action_name))
                    unimplemented!()
                })
        })
    )
}
pub fn handle_refund_action<T: MetadataStore<SwapId>, S: StateStore>(
    metadata_store: &T,
    state_store: Arc<S>,
    id: &SwapId,
    query_params: &ActionExecutionParameters,
) -> Result<ActionResponseBody, HttpApiProblem> {
    let metadata = metadata_store
        .get(id)?
        .ok_or_else(problem::swap_not_found)?;

    with_swap_types!(
        &metadata,
        (|| {
            let state = state_store
                .get::<ROLE>(id.clone())?
                .ok_or_else(problem::state_store)?;
            log::trace!("Retrieved state for {}: {:?}", id, state);

            state
                .actions()
                .into_iter()
                .find_map(|action| match action {
                    Refund(action) => Some(action.into_response_payload(query_params.clone())),
                    _ => None,
                })
                .unwrap_or_else(|| {
                    //                Err(problem::invalid_action(action_name))
                    unimplemented!()
                })
        })
    )
}
pub fn handle_redeem_action<T: MetadataStore<SwapId>, S: StateStore>(
    metadata_store: &T,
    state_store: Arc<S>,
    id: &SwapId,
    query_params: &ActionExecutionParameters,
) -> Result<ActionResponseBody, HttpApiProblem> {
    let metadata = metadata_store
        .get(id)?
        .ok_or_else(problem::swap_not_found)?;

    with_swap_types!(
        &metadata,
        (|| {
            let state = state_store
                .get::<ROLE>(id.clone())?
                .ok_or_else(problem::state_store)?;
            log::trace!("Retrieved state for {}: {:?}", id, state);

            state
                .actions()
                .into_iter()
                .find_map(|action| match action {
                    Redeem(action) => Some(action.into_response_payload(query_params.clone())),
                    _ => None,
                })
                .unwrap_or_else(|| {
                    //                Err(problem::invalid_action(action_name))
                    unimplemented!()
                })
        })
    )
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(tag = "type", content = "payload")]
pub enum ActionResponseBody {
    BitcoinSendAmountToAddress {
        to: bitcoin_support::Address,
        amount: BitcoinQuantity,
        network: bitcoin_support::Network,
    },
    BitcoinBroadcastSignedTransaction {
        hex: String,
        network: bitcoin_support::Network,
        min_median_block_time: Option<Timestamp>,
    },
    EthereumDeployContract {
        data: ethereum_support::Bytes,
        amount: EtherQuantity,
        gas_limit: ethereum_support::U256,
        network: ethereum_support::Network,
    },
    EthereumCallContract {
        contract_address: ethereum_support::Address,
        data: ethereum_support::Bytes,
        gas_limit: ethereum_support::U256,
        network: ethereum_support::Network,
        min_block_timestamp: Option<Timestamp>,
    },
}

#[derive(Clone, Deserialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum ActionExecutionParameters {
    BitcoinAddressAndFee {
        address: bitcoin_support::Address,
        fee_per_byte: String,
    },
    None {},
}

impl ActionResponseBody {
    fn bitcoin_broadcast_signed_transaction(transaction: &Transaction, network: Network) -> Self {
        let min_median_block_time = if transaction.lock_time == 0 {
            None
        } else {
            // The first time a tx with lock_time can be broadcasted is when
            // mediantime == locktime + 1
            let min_median_block_time = transaction.lock_time + 1;
            Some(Timestamp::from(min_median_block_time))
        };

        ActionResponseBody::BitcoinBroadcastSignedTransaction {
            hex: serialize_hex(transaction),
            network,
            min_median_block_time,
        }
    }
}

pub trait IntoResponsePayload {
    fn into_response_payload(
        self,
        parameters: ActionExecutionParameters,
    ) -> Result<ActionResponseBody, HttpApiProblem>;
}

impl IntoResponsePayload for bitcoin::SendToAddress {
    fn into_response_payload(
        self,
        query_params: ActionExecutionParameters,
    ) -> Result<ActionResponseBody, HttpApiProblem> {
        match query_params {
            ActionExecutionParameters::None {} => {
                let bitcoin::SendToAddress {
                    to,
                    amount,
                    network,
                } = self;
                Ok(ActionResponseBody::BitcoinSendAmountToAddress {
                    to,
                    amount,
                    network,
                })
            }
            _ => Err(problem::unexpected_query_parameters(
                "bitcoin::SendToAddress",
                vec!["address".into(), "fee_per_byte".into()],
            )),
        }
    }
}

impl ListRequiredFields for bitcoin::SendToAddress {
    fn list_required_fields() -> Vec<siren::Field> {
        vec![]
    }
}

impl IntoResponsePayload for bitcoin::SpendOutput {
    fn into_response_payload(
        self,
        query_params: ActionExecutionParameters,
    ) -> Result<ActionResponseBody, HttpApiProblem> {
        match query_params {
            ActionExecutionParameters::BitcoinAddressAndFee {
                address,
                fee_per_byte,
            } => {
                let fee_per_byte = fee_per_byte.parse::<f64>().map_err(|_| {
                    HttpApiProblem::new("Invalid query parameter.")
                        .set_status(StatusCode::BAD_REQUEST)
                        .set_detail("Query parameter fee-per-byte is not a valid float.")
                })?;

                let network = self.network;
                let transaction = self.spend_to(address)
                    .sign_with_rate(fee_per_byte)
                    .map_err(|e| {
                        log::error!("Could not sign Bitcoin transaction: {:?}", e);
                        HttpApiProblem::with_title_and_type_from_status(StatusCode::INTERNAL_SERVER_ERROR)
                            .set_detail("Issue encountered when signing Bitcoin transaction.")
                    })?;

                Ok(ActionResponseBody::bitcoin_broadcast_signed_transaction(&transaction, network))
            }
            _ => {
                Err(problem::missing_query_parameters("bitcoin::SpendOutput", vec![
                        &problem::MissingQueryParameter {
                            name: "address",
                            data_type: "string",
                            description: "The bitcoin address to where the funds should be sent.",
                        },
                        &problem::MissingQueryParameter {
                            name: "fee_per_byte",
                            data_type: "float",
                            description:
                            "The fee-per-byte you want to pay for the redeem transaction in satoshis.",
                        },
                ]))
            }
        }
    }
}

impl ListRequiredFields for bitcoin::SpendOutput {
    fn list_required_fields() -> Vec<siren::Field> {
        vec![
            siren::Field {
                name: "address".to_owned(),
                class: vec!["bitcoin".to_owned(), "address".to_owned()],
                _type: Some("text".to_owned()),
                value: None,
                title: None,
            },
            siren::Field {
                name: "fee_per_byte".to_owned(),
                class: vec!["bitcoin".to_owned(), "feePerByte".to_owned()],
                _type: Some("number".to_owned()),
                value: None,
                title: None,
            },
        ]
    }
}

impl IntoResponsePayload for ethereum::ContractDeploy {
    fn into_response_payload(
        self,
        query_params: ActionExecutionParameters,
    ) -> Result<ActionResponseBody, HttpApiProblem> {
        let ethereum::ContractDeploy {
            data,
            amount,
            gas_limit,
            network,
        } = self;
        match query_params {
            ActionExecutionParameters::None {} => Ok(ActionResponseBody::EthereumDeployContract {
                data,
                amount,
                gas_limit,
                network,
            }),
            _ => Err(problem::unexpected_query_parameters(
                "ethereum::ContractDeploy",
                vec!["address".into(), "fee_per_byte".into()],
            )),
        }
    }
}

impl ListRequiredFields for ethereum::ContractDeploy {
    fn list_required_fields() -> Vec<siren::Field> {
        vec![]
    }
}

impl IntoResponsePayload for ethereum::CallContract {
    fn into_response_payload(
        self,
        query_params: ActionExecutionParameters,
    ) -> Result<ActionResponseBody, HttpApiProblem> {
        let ethereum::CallContract {
            to,
            data,
            gas_limit,
            network,
            min_block_timestamp,
        } = self;
        match query_params {
            ActionExecutionParameters::None {} => Ok(ActionResponseBody::EthereumCallContract {
                contract_address: to,
                data,
                gas_limit,
                network,
                min_block_timestamp,
            }),
            _ => Err(problem::unexpected_query_parameters(
                "ethereum::SendTransaction",
                vec!["address".into(), "fee_per_byte".into()],
            )),
        }
    }
}

impl ListRequiredFields for ethereum::CallContract {
    fn list_required_fields() -> Vec<siren::Field> {
        vec![]
    }
}

impl ListRequiredFields for Infallible {
    fn list_required_fields() -> Vec<siren::Field> {
        unreachable!("how did you manage to construct Infallible?")
    }
}

impl IntoResponsePayload for Infallible {
    fn into_response_payload(
        self,
        _: ActionExecutionParameters,
    ) -> Result<ActionResponseBody, HttpApiProblem> {
        unreachable!("how did you manage to construct Infallible?")
    }
}

impl<Deploy, Fund, Redeem, Refund> IntoResponsePayload
    for alice::ActionKind<Deploy, Fund, Redeem, Refund>
where
    Deploy: IntoResponsePayload,
    Fund: IntoResponsePayload,
    Redeem: IntoResponsePayload,
    Refund: IntoResponsePayload,
{
    fn into_response_payload(
        self,
        query_params: ActionExecutionParameters,
    ) -> Result<ActionResponseBody, HttpApiProblem> {
        match self {
            alice::ActionKind::Deploy(payload) => payload.into_response_payload(query_params),
            alice::ActionKind::Fund(payload) => payload.into_response_payload(query_params),
            alice::ActionKind::Redeem(payload) => payload.into_response_payload(query_params),
            alice::ActionKind::Refund(payload) => payload.into_response_payload(query_params),
        }
    }
}

impl<Deploy, Fund, Redeem, Refund> ToSirenAction for alice::ActionKind<Deploy, Fund, Redeem, Refund>
where
    Deploy: ListRequiredFields,
    Fund: ListRequiredFields,
    Redeem: ListRequiredFields,
    Refund: ListRequiredFields,
{
    fn to_siren_action(&self, id: &SwapId) -> siren::Action {
        let (name, fields) = match self {
            alice::ActionKind::Deploy(_) => ("deploy", Deploy::list_required_fields()),
            alice::ActionKind::Fund(_) => ("fund", Fund::list_required_fields()),
            alice::ActionKind::Redeem(_) => ("redeem", Redeem::list_required_fields()),
            alice::ActionKind::Refund(_) => ("refund", Refund::list_required_fields()),
        };

        siren::Action {
            name: name.to_owned(),
            href: new_action_link(id, name),
            method: Some(http::Method::GET),
            _type: Some("application/x-www-form-urlencoded".to_owned()),
            fields,
            class: vec![],
            title: None,
        }
    }
}

impl<Accept, Decline, Deploy, Fund, Redeem, Refund> ToSirenAction
    for bob::ActionKind<Accept, Decline, Deploy, Fund, Redeem, Refund>
where
    Accept: ToSirenAction,
    Deploy: ListRequiredFields,
    Fund: ListRequiredFields,
    Redeem: ListRequiredFields,
    Refund: ListRequiredFields,
{
    fn to_siren_action(&self, id: &SwapId) -> siren::Action {
        let (name, fields) = match self {
            bob::ActionKind::Deploy(_) => ("deploy", Deploy::list_required_fields()),
            bob::ActionKind::Fund(_) => ("fund", Fund::list_required_fields()),
            bob::ActionKind::Redeem(_) => ("redeem", Redeem::list_required_fields()),
            bob::ActionKind::Refund(_) => ("refund", Refund::list_required_fields()),
            bob::ActionKind::Decline(_) => {
                return siren::Action {
                    name: "decline".to_owned(),
                    href: new_action_link(id, "decline"),
                    method: Some(http::Method::POST),
                    _type: Some("application/json".to_owned()),
                    fields: vec![],
                    class: vec![],
                    title: None,
                }
            }
            bob::ActionKind::Accept(accept) => return accept.to_siren_action(id),
        };

        siren::Action {
            name: name.to_owned(),
            href: new_action_link(id, name),
            method: Some(http::Method::GET),
            _type: Some("application/x-www-form-urlencoded".to_owned()),
            fields,
            class: vec![],
            title: None,
        }
    }
}

impl<Accept, Decline, Deploy, Fund, Redeem, Refund> IntoResponsePayload
    for bob::ActionKind<Accept, Decline, Deploy, Fund, Redeem, Refund>
where
    Deploy: IntoResponsePayload,
    Fund: IntoResponsePayload,
    Redeem: IntoResponsePayload,
    Refund: IntoResponsePayload,
{
    fn into_response_payload(
        self,
        query_params: ActionExecutionParameters,
    ) -> Result<ActionResponseBody, HttpApiProblem> {
        match self {
            bob::ActionKind::Deploy(payload) => payload.into_response_payload(query_params),
            bob::ActionKind::Fund(payload) => payload.into_response_payload(query_params),
            bob::ActionKind::Redeem(payload) => payload.into_response_payload(query_params),
            bob::ActionKind::Refund(payload) => payload.into_response_payload(query_params),
            bob::ActionKind::Accept(_) | bob::ActionKind::Decline(_) => {
                log::error!("IntoResponsePayload is not implemented for Accept/Decline");
                Err(HttpApiProblem::with_title_and_type_from_status(
                    StatusCode::INTERNAL_SERVER_ERROR,
                ))
            }
        }
    }
}
