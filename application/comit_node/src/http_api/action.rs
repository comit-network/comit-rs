use crate::{
    http_api::problem,
    swap_protocols::{
        actions::{bitcoin, ethereum},
        SwapId, Timestamp,
    },
};
use http::StatusCode;
use http_api_problem::HttpApiProblem;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;

pub trait ToSirenAction {
    fn to_siren_action(&self, id: &SwapId) -> siren::Action;
}

pub trait ListRequiredFields {
    fn list_required_fields() -> Vec<siren::Field>;
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

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(tag = "type", content = "payload")]
pub enum ActionResponseBody {
    BitcoinSendAmountToAddress {
        to: bitcoin_support::Address,
        amount: bitcoin_support::BitcoinQuantity,
        network: bitcoin_support::Network,
    },
    BitcoinBroadcastSignedTransaction {
        hex: String,
        network: bitcoin_support::Network,
        min_median_block_time: Option<Timestamp>,
    },
    EthereumDeployContract {
        data: ethereum_support::Bytes,
        amount: ethereum_support::EtherQuantity,
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
    None,
}

impl ActionResponseBody {
    fn bitcoin_broadcast_signed_transaction(
        transaction: &bitcoin_support::Transaction,
        network: bitcoin_support::Network,
    ) -> Self {
        let min_median_block_time = if transaction.lock_time == 0 {
            None
        } else {
            // The first time a tx with lock_time can be broadcasted is when
            // mediantime == locktime + 1
            let min_median_block_time = transaction.lock_time + 1;
            Some(Timestamp::from(min_median_block_time))
        };

        ActionResponseBody::BitcoinBroadcastSignedTransaction {
            hex: bitcoin_support::serialize_hex(transaction),
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
                let transaction = self
                    .spend_to(address)
                    .sign_with_rate(fee_per_byte)
                    .map_err(|e| {
                        log::error!("Could not sign Bitcoin transaction: {:?}", e);
                        HttpApiProblem::with_title_and_type_from_status(
                            StatusCode::INTERNAL_SERVER_ERROR,
                        )
                        .set_detail("Issue encountered when signing Bitcoin transaction.")
                    })?;

                Ok(ActionResponseBody::bitcoin_broadcast_signed_transaction(
                    &transaction,
                    network,
                ))
            }
            _ => Err(problem::missing_query_parameters(
                "bitcoin::SpendOutput",
                vec![
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
                ],
            )),
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

impl IntoResponsePayload for ethereum::DeployContract {
    fn into_response_payload(
        self,
        query_params: ActionExecutionParameters,
    ) -> Result<ActionResponseBody, HttpApiProblem> {
        let ethereum::DeployContract {
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

impl ListRequiredFields for ethereum::DeployContract {
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn given_no_query_parameters_deserialize_to_none() {
        let s = "";

        let res = serde_urlencoded::from_str::<ActionExecutionParameters>(s);
        assert_eq!(res, Ok(ActionExecutionParameters::None {}));
    }

    #[test]
    fn given_bitcoin_identity_and_fee_deserialize_to_ditto() {
        let s = "address=1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa&fee_per_byte=10.59";

        let res = serde_urlencoded::from_str::<ActionExecutionParameters>(s);
        assert_eq!(
            res,
            Ok(ActionExecutionParameters::BitcoinAddressAndFee {
                address: "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".parse().unwrap(),
                fee_per_byte: "10.59".to_string(),
            })
        );
    }
}
