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
        fee_per_wu: String,
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
        #[serde(skip_serializing_if = "Option::is_none")]
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
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<ethereum_support::Bytes>,
        gas_limit: ethereum_support::U256,
        network: ethereum_support::Network,
        #[serde(skip_serializing_if = "Option::is_none")]
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
                vec!["address".into(), "fee_per_wu".into()],
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
                fee_per_wu,
            } => {
                let fee_per_wu = fee_per_wu.parse::<u64>().map_err(|_| {
                    HttpApiProblem::new("Invalid query parameter.")
                        .set_status(StatusCode::BAD_REQUEST)
                        .set_detail("Query parameter fee-per-byte is not a valid unsigned integer.")
                })?;

                let network = self.network;
                let transaction = unimplemented!();
                //                    self.spend_to(address)
                //                        .sign_with_rate(fee_per_wu)
                //                        .map_err(|e| {
                //                            log::error!("Could not sign Bitcoin transaction:
                // {:?}", e);                            match e {
                //
                // bitcoin_witness::Error::FeeHigherThanInputValue => HttpApiProblem::new(
                //                                    "Fee is too high.",
                //                                )
                //                                .set_status(StatusCode::BAD_REQUEST)
                //                                .set_detail(
                //                                    "The Fee per byte/WU provided makes the
                // total fee higher than the spendable input value.",
                //                                ),
                //                                bitcoin_witness::Error::OverflowingFee =>
                // HttpApiProblem::new(                                    "Fee
                // is too high.",                                )
                //                                    .set_status(StatusCode::BAD_REQUEST)
                //                                    .set_detail(
                //                                        "The Fee per byte/WU provided makes
                // the total fee higher than the system supports.",
                // )                            }
                //                        })?;

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
                        name: "fee_per_wu",
                        data_type: "uint",
                        description:
                        "The fee per weight unit you want to pay for the transaction in satoshis.",
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
                name: "fee_per_wu".to_owned(),
                class: vec![
                    "bitcoin".to_owned(),
                    // feePerByte is deprecated because it is actually fee per WU
                    // Have to keep it around until clients are upgraded
                    "feePerByte".to_owned(),
                    "feePerWU".to_owned(),
                ],
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
                vec!["address".into(), "fee_per_wu".into()],
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
                vec!["address".into(), "fee_per_wu".into()],
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
    use ethereum_support::*;
    use std::str::FromStr;

    #[test]
    fn given_no_query_parameters_deserialize_to_none() {
        let s = "";

        let res = serde_urlencoded::from_str::<ActionExecutionParameters>(s);
        assert_eq!(res, Ok(ActionExecutionParameters::None {}));
    }

    #[test]
    fn given_bitcoin_identity_and_fee_deserialize_to_ditto() {
        let s = "address=1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa&fee_per_wu=10.59";

        let res = serde_urlencoded::from_str::<ActionExecutionParameters>(s);
        assert_eq!(
            res,
            Ok(ActionExecutionParameters::BitcoinAddressAndFee {
                address: "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".parse().unwrap(),
                fee_per_wu: "10.59".to_string(),
            })
        );
    }

    #[test]
    fn call_contract_serializes_correctly_to_json_with_none() {
        let addr = Address::from_str("0A81e8be41b21f651a71aaB1A85c6813b8bBcCf8").unwrap();
        let contract = ActionResponseBody::EthereumCallContract {
            contract_address: addr,
            data: None,
            gas_limit: U256::from(1),
            network: Network::Ropsten,
            min_block_timestamp: None,
        };
        let serialized = serde_json::to_string(&contract).unwrap();
        assert_eq!(
            serialized,
            r#"{"type":"ethereum-call-contract","payload":{"contract_address":"0x0a81e8be41b21f651a71aab1a85c6813b8bbccf8","gas_limit":"0x1","network":"ropsten"}}"#
        );
    }
}
