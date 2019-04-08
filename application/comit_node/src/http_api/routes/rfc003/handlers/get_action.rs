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
use bitcoin_support::{self, serialize::serialize_hex, BitcoinQuantity};
use ethereum_support::{self, Erc20Token, EtherQuantity};
use http_api_problem::{HttpApiProblem, StatusCode};
use rustic_hal::HalResource;
use std::sync::Arc;

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
            trace!("Retrieved state for {}: {:?}", id, state);

            state
                .actions()
                .iter()
                .find_map(|action| {
                    if action_name == action.to_action_name() {
                        let payload = action
                            .inner
                            .clone()
                            .into_response_payload(query_params.clone());

                        match payload {
                            Ok(payload) => {
                                trace!(
                                    "Swap {}: Returning {:?} for {:?}",
                                    id,
                                    payload,
                                    action_name
                                );
                                Some(Ok(HalResource::new(ActionResponseBody {
                                    payload,
                                    invalid_until: action.invalid_until,
                                })))
                            }
                            Err(e) => Some(Err(e)),
                        }
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
    ) -> Result<ActionResponsePayload, HttpApiProblem>;
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(tag = "type", content = "payload")]
pub enum ActionResponsePayload {
    BitcoinSendAmountToAddress {
        to: bitcoin_support::Address,
        amount: BitcoinQuantity,
        network: bitcoin_support::Network,
    },
    BitcoinBroadcastSignedTransaction {
        hex: String,
        network: bitcoin_support::Network,
    },
    EthereumDeployContract {
        data: ethereum_support::Bytes,
        amount: EtherQuantity,
        gas_limit: ethereum_support::U256,
        network: ethereum_support::Network,
    },
    EthereumInvokeContract {
        contract_address: ethereum_support::Address,
        data: ethereum_support::Bytes,
        amount: EtherQuantity,
        gas_limit: ethereum_support::U256,
        network: ethereum_support::Network,
    },
}

#[derive(Clone, Debug, Serialize)]
pub struct ActionResponseBody {
    #[serde(flatten)]
    pub payload: ActionResponsePayload,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invalid_until: Option<Timestamp>,
}

#[derive(Serialize)]
struct MissingQueryParameter {
    data_type: &'static str,
    description: &'static str,
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
    ) -> Result<ActionResponsePayload, HttpApiProblem> {
        match query_params {
            GetActionQueryParams::None {} => {
                let bitcoin::SendToAddress {
                    to,
                    amount,
                    network,
                } = self;
                Ok(ActionResponsePayload::BitcoinSendAmountToAddress {
                    to,
                    amount,
                    network,
                })
            }
            _ => {
                error!("Unexpected GET parameters for a bitcoin::SendToAddress action type. Expected: none.");
                Err(
                    HttpApiProblem::with_title_and_type_from_status(StatusCode::BAD_REQUEST)
                        .set_detail("This action does not take any query parameters"),
                )
            }
        }
    }
}

impl IntoResponsePayload for bitcoin::SpendOutput {
    fn into_response_payload(
        self,
        query_params: GetActionQueryParams,
    ) -> Result<ActionResponsePayload, HttpApiProblem> {
        match query_params {
            GetActionQueryParams::BitcoinAddressAndFee {
                address,
                fee_per_byte,
            } => match fee_per_byte.parse::<f64>() {
                Ok(fee_per_byte) => {
                    let network = self.network;
                    let transaction = self.spend_to(address).sign_with_rate(fee_per_byte);
                    let transaction = match transaction {
                        Ok(transaction) => transaction,
                        Err(e) => {
                            error!("Could not sign Bitcoin transaction: {:?}", e);
                            return Err(HttpApiProblem::with_title_and_type_from_status(
                                StatusCode::INTERNAL_SERVER_ERROR,
                            )
                            .set_detail("Issue encountered when signing Bitcoin transaction"));
                        }
                    };
                    match serialize_hex(&transaction) {
                        Ok(hex) => Ok(ActionResponsePayload::BitcoinBroadcastSignedTransaction {
                            hex,
                            network,
                        }),
                        Err(e) => {
                            error!("Could not serialized signed Bitcoin transaction: {:?}", e);
                            Err(HttpApiProblem::with_title_and_type_from_status(
                                StatusCode::INTERNAL_SERVER_ERROR,
                            )
                            .set_detail("Issue encountered when serializing Bitcoin transaction"))
                        }
                    }
                }
                Err(_) => Err(HttpApiProblem::with_title_and_type_from_status(
                    StatusCode::BAD_REQUEST,
                )
                .set_detail("fee-per-byte is not a valid float")),
            },
            _ => {
                error!("Unexpected GET parameters for a bitcoin::SpendOutput action type. Expected: address and fee-per-byte.");
                let mut problem =
                    HttpApiProblem::with_title_and_type_from_status(StatusCode::BAD_REQUEST)
                        .set_detail("This action requires additional query parameters");
                problem
                    .set_value(
                        "address",
                        &MissingQueryParameter {
                            data_type: "string",
                            description: "The bitcoin address to where the funds should be sent",
                        },
                    )
                    .expect("invalid use of HttpApiProblem");
                problem
                    .set_value(
                        "fee_per_byte",
                        &MissingQueryParameter {
                            data_type: "float",
                            description:
                            "The fee-per-byte you want to pay for the redeem transaction in satoshis",
                        },
                    )
                    .expect("invalid use of HttpApiProblem");

                Err(problem)
            }
        }
    }
}

impl IntoResponsePayload for ethereum::ContractDeploy {
    fn into_response_payload(
        self,
        query_params: GetActionQueryParams,
    ) -> Result<ActionResponsePayload, HttpApiProblem> {
        let ethereum::ContractDeploy {
            data,
            amount,
            gas_limit,
            network,
        } = self;
        match query_params {
            GetActionQueryParams::None {} => Ok(ActionResponsePayload::EthereumDeployContract {
                data,
                amount,
                gas_limit,
                network,
            }),
            _ => {
                error!("Unexpected GET parameters for an ethereum::ContractDeploy action type. Expected: None.");
                Err(
                    HttpApiProblem::with_title_and_type_from_status(StatusCode::BAD_REQUEST)
                        .set_detail("This action does not take any query parameters"),
                )
            }
        }
    }
}

impl IntoResponsePayload for ethereum::SendTransaction {
    fn into_response_payload(
        self,
        query_params: GetActionQueryParams,
    ) -> Result<ActionResponsePayload, HttpApiProblem> {
        let ethereum::SendTransaction {
            to,
            data,
            amount,
            gas_limit,
            network,
        } = self;
        match query_params {
            GetActionQueryParams::None {} => Ok(ActionResponsePayload::EthereumInvokeContract {
                contract_address: to,
                data,
                amount,
                gas_limit,
                network,
            }),
            _ => {
                error!("Unexpected GET parameters for an ethereum::SendTransaction action. Expected: None.");
                Err(
                    HttpApiProblem::with_title_and_type_from_status(StatusCode::BAD_REQUEST)
                        .set_detail("This action does not take any query parameters"),
                )
            }
        }
    }
}

impl IntoResponsePayload for () {
    fn into_response_payload(
        self,
        _: GetActionQueryParams,
    ) -> Result<ActionResponsePayload, HttpApiProblem> {
        error!("IntoResponsePayload should not be called for the unit type");
        Err(HttpApiProblem::with_title_and_type_from_status(
            StatusCode::INTERNAL_SERVER_ERROR,
        ))
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
    ) -> Result<ActionResponsePayload, HttpApiProblem> {
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
    ) -> Result<ActionResponsePayload, HttpApiProblem> {
        match self {
            bob::ActionKind::Deploy(payload) => payload.into_response_payload(query_params),
            bob::ActionKind::Fund(payload) => payload.into_response_payload(query_params),
            bob::ActionKind::Redeem(payload) => payload.into_response_payload(query_params),
            bob::ActionKind::Refund(payload) => payload.into_response_payload(query_params),
            _ => {
                error!("IntoResponsePayload is not implemented for Accept/Decline");
                Err(HttpApiProblem::with_title_and_type_from_status(
                    StatusCode::INTERNAL_SERVER_ERROR,
                ))
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn given_no_query_parameters_deserialize_to_none() {
        let s = "";

        let res = serde_urlencoded::from_str::<GetActionQueryParams>(s);
        assert_eq!(res, Ok(GetActionQueryParams::None {}));
    }

    #[test]
    fn given_bitcoin_identity_and_fee_deserialize_to_ditto() {
        let s = "address=1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa&fee_per_byte=10.59";

        let res = serde_urlencoded::from_str::<GetActionQueryParams>(s);
        assert_eq!(
            res,
            Ok(GetActionQueryParams::BitcoinAddressAndFee {
                address: "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".parse().unwrap(),
                fee_per_byte: "10.59".to_string(),
            })
        );
    }
}
