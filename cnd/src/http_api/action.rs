use crate::{
    actions::{
        bitcoin::{self, SendToAddress, SpendOutput},
        ethereum, lnd,
        lnd::Chain,
    },
    asset,
    ethereum::ChainId,
    http_api::{problem, Http, MissingQueryParameters, UnexpectedQueryParameters},
    identity, transaction, LocalSwapId, RelativeTime, Secret, SecretHash, Timestamp,
};
use anyhow::Context;
use blockchain_contracts::bitcoin::witness;
use http_api_problem::HttpApiProblem;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use warp::http::StatusCode;

pub trait ToSirenAction {
    fn to_siren_action(&self, id: &LocalSwapId) -> siren::Action;
}

pub trait ListRequiredFields {
    fn list_required_fields() -> Vec<siren::Field>;
}

#[derive(Clone, Deserialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum ActionExecutionParameters {
    BitcoinAddressAndFee {
        address: bitcoin::Address,
        fee_per_wu: String,
    },
    None {},
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(tag = "type", content = "payload")]
pub enum ActionResponseBody {
    BitcoinSendAmountToAddress {
        to: bitcoin::Address,
        amount: String,
        network: Http<bitcoin::Network>,
    },
    BitcoinBroadcastSignedTransaction {
        hex: String,
        network: Http<bitcoin::Network>,
        #[serde(skip_serializing_if = "Option::is_none")]
        min_median_block_time: Option<Timestamp>,
    },
    EthereumDeployContract {
        data: EthereumData,
        amount: asset::Ether,
        gas_limit: crate::ethereum::U256,
        chain_id: ChainId,
    },
    EthereumCallContract {
        contract_address: identity::Ethereum,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<EthereumData>,
        gas_limit: crate::ethereum::U256,
        chain_id: ChainId,
        #[serde(skip_serializing_if = "Option::is_none")]
        min_block_timestamp: Option<Timestamp>,
    },
    LndAddHoldInvoice {
        #[serde(with = "asset::bitcoin::sats_as_string")]
        amount: asset::Bitcoin,
        secret_hash: SecretHash,
        expiry: RelativeTime,
        cltv_expiry: RelativeTime,
        chain: Http<Chain>,
        network: Http<bitcoin::Network>,
        self_public_key: identity::Lightning,
    },
    LndSendPayment {
        to_public_key: identity::Lightning,
        #[serde(with = "asset::bitcoin::sats_as_string")]
        amount: asset::Bitcoin,
        secret_hash: SecretHash,
        final_cltv_delta: RelativeTime,
        chain: Http<Chain>,
        network: Http<bitcoin::Network>,
        self_public_key: identity::Lightning,
    },
    LndSettleInvoice {
        secret: Secret,
        chain: Http<Chain>,
        network: Http<bitcoin::Network>,
        self_public_key: identity::Lightning,
    },
    None,
}

/// A wrapper type for serializing bytes to hex with a `0x` prefix.
///
/// In the Ethereum ecosystem (i.e. Web3-based clients), the `data` field of a
/// transaction is serialized to hex with a `0x` prefix when represented as a
/// string.
/// We want our API to be easily interoperable with such clients, hence we
/// serialize this data already in that format so consumers of our API can
/// simply pass it along to a Web3-based client.
#[derive(Clone, Debug, Serialize)]
pub struct EthereumData(#[serde(with = "serde_hex::SerHexSeq::<serde_hex::StrictPfx>")] Vec<u8>);

impl<T: Into<Vec<u8>>> From<T> for EthereumData {
    fn from(data: T) -> Self {
        EthereumData(data.into())
    }
}

impl ActionResponseBody {
    pub fn bitcoin_broadcast_signed_transaction(
        transaction: &transaction::Bitcoin,
        network: bitcoin::Network,
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
            hex: ::bitcoin::consensus::encode::serialize_hex(transaction),
            network: Http(network),
            min_median_block_time,
        }
    }
}

pub trait IntoResponsePayload {
    fn into_response_payload(
        self,
        parameters: ActionExecutionParameters,
    ) -> anyhow::Result<ActionResponseBody>;
}

impl IntoResponsePayload for SendToAddress {
    fn into_response_payload(
        self,
        query_params: ActionExecutionParameters,
    ) -> anyhow::Result<ActionResponseBody> {
        match query_params {
            ActionExecutionParameters::None {} => Ok(self.into()),
            _ => Err(anyhow::Error::from(UnexpectedQueryParameters {
                action: "bitcoin::SendToAddress",
                parameters: &["address", "fee_per_wu"],
            })),
        }
    }
}

impl From<bitcoin::SendToAddress> for ActionResponseBody {
    fn from(action: SendToAddress) -> Self {
        let SendToAddress {
            to,
            amount,
            network,
        } = action;
        ActionResponseBody::BitcoinSendAmountToAddress {
            to,
            amount: amount.as_sat().to_string(),
            network: Http(network),
        }
    }
}

impl From<bitcoin::BroadcastSignedTransaction> for ActionResponseBody {
    fn from(
        bitcoin::BroadcastSignedTransaction {
            transaction,
            network,
        }: bitcoin::BroadcastSignedTransaction,
    ) -> Self {
        Self::bitcoin_broadcast_signed_transaction(&transaction, network)
    }
}

impl From<lnd::AddHoldInvoice> for ActionResponseBody {
    fn from(action: lnd::AddHoldInvoice) -> Self {
        let lnd::AddHoldInvoice {
            amount,
            secret_hash,
            expiry,
            cltv_expiry,
            chain,
            network,
            self_public_key,
        } = action;

        ActionResponseBody::LndAddHoldInvoice {
            amount,
            secret_hash,
            expiry,
            cltv_expiry,
            chain: Http(chain),
            network: Http(network),
            self_public_key,
        }
    }
}

impl From<ethereum::DeployContract> for ActionResponseBody {
    fn from(action: ethereum::DeployContract) -> Self {
        let ethereum::DeployContract {
            amount,
            chain_id,
            gas_limit,
            data,
        } = action;

        ActionResponseBody::EthereumDeployContract {
            data: EthereumData(data),
            amount,
            gas_limit: gas_limit.into(),
            chain_id,
        }
    }
}

impl From<lnd::SendPayment> for ActionResponseBody {
    fn from(action: lnd::SendPayment) -> Self {
        let lnd::SendPayment {
            to_public_key,
            amount,
            secret_hash,
            network,
            chain,
            final_cltv_delta,
            self_public_key,
        } = action;

        ActionResponseBody::LndSendPayment {
            to_public_key,
            amount,
            secret_hash,
            network: network.into(),
            chain: chain.into(),
            final_cltv_delta,
            self_public_key,
        }
    }
}

impl From<lnd::SettleInvoice> for ActionResponseBody {
    fn from(action: lnd::SettleInvoice) -> Self {
        let lnd::SettleInvoice {
            secret,
            chain,
            network,
            self_public_key,
        } = action;

        ActionResponseBody::LndSettleInvoice {
            secret,
            chain: chain.into(),
            network: network.into(),
            self_public_key,
        }
    }
}

impl From<ethereum::CallContract> for ActionResponseBody {
    fn from(action: ethereum::CallContract) -> Self {
        let ethereum::CallContract {
            to,
            data,
            gas_limit,
            chain_id,
            min_block_timestamp,
        } = action;

        ActionResponseBody::EthereumCallContract {
            contract_address: to,
            data: data.map(EthereumData),
            gas_limit: gas_limit.into(),
            chain_id,
            min_block_timestamp,
        }
    }
}

impl From<comit::Never> for ActionResponseBody {
    fn from(_: comit::Never) -> Self {
        unreachable!("impl should be removed once ! type is stabilised")
    }
}

impl ListRequiredFields for SendToAddress {
    fn list_required_fields() -> Vec<siren::Field> {
        vec![]
    }
}

impl IntoResponsePayload for SpendOutput {
    fn into_response_payload(
        self,
        query_params: ActionExecutionParameters,
    ) -> anyhow::Result<ActionResponseBody> {
        match query_params {
            ActionExecutionParameters::BitcoinAddressAndFee {
                address,
                fee_per_wu,
            } => {
                let fee_per_wu = fee_per_wu.parse::<usize>().with_context(|| {
                    HttpApiProblem::new("Invalid query parameter.")
                        .set_status(StatusCode::BAD_REQUEST)
                        .set_detail("Query parameter fee-per-byte is not a valid unsigned integer.")
                })?;

                let network = self.network;
                let transaction =
                    self.spend_to(address)
                        .sign_with_rate(&*crate::SECP, fee_per_wu)
                        .map_err(|e| {
                            tracing::error!("Could not sign Bitcoin transaction: {:?}", e);
                            match e {
                                witness::Error::FeeHigherThanInputValue => HttpApiProblem::new(
                                    "Fee is too high.",
                                )
                                .set_status(StatusCode::BAD_REQUEST)
                                .set_detail(
                                    "The Fee per byte/WU provided makes the total fee higher than the spendable input value.",
                                ),
                                witness::Error::OverflowingFee => HttpApiProblem::new(
                                    "Fee is too high.",
                                )
                                    .set_status(StatusCode::BAD_REQUEST)
                                    .set_detail(
                                        "The Fee per byte/WU provided makes the total fee higher than the system supports.",
                                    )
                            }
                        })?;

                Ok(ActionResponseBody::bitcoin_broadcast_signed_transaction(
                    &transaction,
                    network,
                ))
            }
            _ => Err(anyhow::Error::from(MissingQueryParameters {
                action: "bitcoin::SpendOutput",
                parameters: &[
                    problem::MissingQueryParameter {
                        name: "address",
                        data_type: "string",
                        description: "The bitcoin address to where the funds should be sent.",
                    },
                    problem::MissingQueryParameter {
                        name: "fee_per_wu",
                        data_type: "uint",
                        description:
                        "The fee per weight unit you want to pay for the transaction in satoshis.",
                    },
                ]
            }))
        }
    }
}

impl ListRequiredFields for SpendOutput {
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
    ) -> anyhow::Result<ActionResponseBody> {
        match query_params {
            ActionExecutionParameters::None {} => Ok(self.into()),
            _ => Err(anyhow::Error::from(UnexpectedQueryParameters {
                action: "ethereum::ContractDeploy",
                parameters: &["address", "fee_per_wu"],
            })),
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
    ) -> anyhow::Result<ActionResponseBody> {
        match query_params {
            ActionExecutionParameters::None {} => Ok(self.into()),
            _ => Err(anyhow::Error::from(UnexpectedQueryParameters {
                action: "ethereum::SendTransaction",
                parameters: &["address", "fee_per_wu"],
            })),
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
    ) -> anyhow::Result<ActionResponseBody> {
        unreachable!("how did you manage to construct Infallible?")
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        asset::ethereum::FromWei, bitcoin::Address as BitcoinAddress, ethereum::U256, identity,
    };
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
        let addr =
            identity::Ethereum::from_str("0A81e8be41b21f651a71aaB1A85c6813b8bBcCf8").unwrap();
        let chain_id = ChainId::from(3);
        let contract = ActionResponseBody::EthereumCallContract {
            contract_address: addr,
            data: None,
            gas_limit: U256::from(1),
            chain_id,
            min_block_timestamp: None,
        };
        let serialized = serde_json::to_string(&contract).unwrap();
        assert_eq!(
            serialized,
            r#"{"type":"ethereum-call-contract","payload":{"contract_address":"0x0a81e8be41b21f651a71aab1a85c6813b8bbccf8","gas_limit":"0x1","chain_id":3}}"#
        );
    }

    #[test]
    fn deploy_contract_serializes_correctly_to_json() {
        let response_body = ActionResponseBody::EthereumDeployContract {
            data: EthereumData::from(vec![0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8, 0x9, 0x10]),
            amount: asset::Ether::from_wei(10000u32),
            gas_limit: U256::from(1),
            chain_id: ChainId::from(3),
        };

        let serialized = serde_json::to_string(&response_body).unwrap();

        assert_eq!(
            serialized,
            r#"{"type":"ethereum-deploy-contract","payload":{"data":"0x01020304050607080910","amount":"10000","gas_limit":"0x1","chain_id":3}}"#
        );
    }

    #[test]
    fn bitcoin_send_amount_to_address_serializes_correctly_to_json() {
        let to = BitcoinAddress::from_str("2N3pk6v15FrDiRNKYVuxnnugn1Yg7wfQRL9").unwrap();
        let amount = asset::Bitcoin::from_sat(100_000_000);

        let input = &[
            ActionResponseBody::from(SendToAddress {
                to: to.clone().into(),
                amount,
                network: bitcoin::Network::Bitcoin,
            }),
            ActionResponseBody::from(SendToAddress {
                to: to.clone().into(),
                amount,
                network: bitcoin::Network::Testnet,
            }),
            ActionResponseBody::from(SendToAddress {
                to: to.into(),
                amount,
                network: bitcoin::Network::Regtest,
            }),
        ];

        let expected = &[
            r#"{"type":"bitcoin-send-amount-to-address","payload":{"to":"2N3pk6v15FrDiRNKYVuxnnugn1Yg7wfQRL9","amount":"100000000","network":"mainnet"}}"#,
            r#"{"type":"bitcoin-send-amount-to-address","payload":{"to":"2N3pk6v15FrDiRNKYVuxnnugn1Yg7wfQRL9","amount":"100000000","network":"testnet"}}"#,
            r#"{"type":"bitcoin-send-amount-to-address","payload":{"to":"2N3pk6v15FrDiRNKYVuxnnugn1Yg7wfQRL9","amount":"100000000","network":"regtest"}}"#,
        ];

        let actual = input
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<String>, serde_json::Error>>()
            .unwrap();

        assert_eq!(actual, expected);
    }
}
