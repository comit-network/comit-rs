use crate::{
    http_api::{
        problem,
        routes::rfc003::action::{ActionName, ToActionName},
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
use rustic_hal::HalResource;
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, sync::Arc};

pub fn handle_get_action<T: MetadataStore<SwapId>, S: StateStore>(
    metadata_store: &T,
    state_store: Arc<S>,
    id: &SwapId,
    action_name: ActionName,
    query_params: &GetActionQueryParams,
) -> Result<HalResource, HttpApiProblem> {
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
                .iter()
                .find_map(|action| {
                    if action_name == action.to_action_name() {
                        let payload = action
                            .clone()
                            .into_response_payload(query_params.clone())
                            .map(|payload| {
                                log::trace!(
                                    "Swap {}: Returning {:?} for {:?}",
                                    id,
                                    payload,
                                    action_name
                                );

                                HalResource::new(payload)
                            });

                        Some(payload)
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| Err(problem::invalid_action(action_name)))
        })
    )
}

pub trait IntoResponsePayload {
    fn into_response_payload(
        self,
        query_params: GetActionQueryParams,
    ) -> Result<ActionResponseBody, HttpApiProblem>;
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

#[derive(Clone, Deserialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum GetActionQueryParams {
    BitcoinAddressAndFee {
        address: bitcoin_support::Address,
        fee_per_byte: String,
    },
    None {},
}

impl IntoResponsePayload for bitcoin::SendToAddress {
    fn into_response_payload(
        self,
        query_params: GetActionQueryParams,
    ) -> Result<ActionResponseBody, HttpApiProblem> {
        match query_params {
            GetActionQueryParams::None {} => {
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

impl IntoResponsePayload for bitcoin::SpendOutput {
    fn into_response_payload(
        self,
        query_params: GetActionQueryParams,
    ) -> Result<ActionResponseBody, HttpApiProblem> {
        match query_params {
            GetActionQueryParams::BitcoinAddressAndFee {
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

impl IntoResponsePayload for ethereum::ContractDeploy {
    fn into_response_payload(
        self,
        query_params: GetActionQueryParams,
    ) -> Result<ActionResponseBody, HttpApiProblem> {
        let ethereum::ContractDeploy {
            data,
            amount,
            gas_limit,
            network,
        } = self;
        match query_params {
            GetActionQueryParams::None {} => Ok(ActionResponseBody::EthereumDeployContract {
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

impl IntoResponsePayload for ethereum::CallContract {
    fn into_response_payload(
        self,
        query_params: GetActionQueryParams,
    ) -> Result<ActionResponseBody, HttpApiProblem> {
        let ethereum::CallContract {
            to,
            data,
            gas_limit,
            network,
            min_block_timestamp,
        } = self;
        match query_params {
            GetActionQueryParams::None {} => Ok(ActionResponseBody::EthereumCallContract {
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

impl IntoResponsePayload for Infallible {
    fn into_response_payload(
        self,
        _: GetActionQueryParams,
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
        query_params: GetActionQueryParams,
    ) -> Result<ActionResponseBody, HttpApiProblem> {
        match self {
            alice::ActionKind::Deploy(payload) => payload.into_response_payload(query_params),
            alice::ActionKind::Fund(payload) => payload.into_response_payload(query_params),
            alice::ActionKind::Redeem(payload) => payload.into_response_payload(query_params),
            alice::ActionKind::Refund(payload) => payload.into_response_payload(query_params),
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
        query_params: GetActionQueryParams,
    ) -> Result<ActionResponseBody, HttpApiProblem> {
        match self {
            bob::ActionKind::Deploy(payload) => payload.into_response_payload(query_params),
            bob::ActionKind::Fund(payload) => payload.into_response_payload(query_params),
            bob::ActionKind::Redeem(payload) => payload.into_response_payload(query_params),
            bob::ActionKind::Refund(payload) => payload.into_response_payload(query_params),
            _ => {
                log::error!("IntoResponsePayload is not implemented for Accept/Decline");
                Err(HttpApiProblem::with_title_and_type_from_status(
                    StatusCode::INTERNAL_SERVER_ERROR,
                ))
            }
        }
    }
}
