use crate::{
    http_api::{
        problem,
        rfc003::routes::{GetAction, GetActionQueryParams},
    },
    swap_protocols::{
        ledger::{Bitcoin, Ethereum},
        metadata_store::Metadata,
        rfc003::{alice, bitcoin, bob, ethereum, state_store::StateStore, Actions, Alice, Bob},
        MetadataStore, RoleKind, SwapId,
    },
};
use bitcoin_support::{self, serialize::serialize_hex, BitcoinQuantity};
use ethereum_support::{self, Erc20Token, EtherQuantity};
use http_api_problem::HttpApiProblem;
use std::{str::FromStr, sync::Arc};
use warp::{self, Reply};

pub fn handle_get_action<T: MetadataStore<SwapId>, S: StateStore<SwapId>>(
    metadata_store: &T,
    state_store: Arc<S>,
    id: &SwapId,
    action: GetAction,
    query_params: &GetActionQueryParams,
) -> Result<impl Reply, HttpApiProblem> {
    let metadata = metadata_store
        .get(id)?
        .ok_or_else(problem::swap_not_found)?;

    with_swap_types!(
        &metadata,
        (|| {
            let state = state_store
                .get::<Role>(id)?
                .ok_or_else(problem::state_store)?;
            trace!("Retrieved state for {}: {:?}", id, state);

            state
                .actions()
                .iter()
                .find_map(|state_action| {
                    if action.matches(state_action) {
                        Some(
                            state_action
                                .clone()
                                .into_response_body(query_params.clone())
                                .map(|body| {
                                    trace!("Swap {}: Returning {:?} for {:?}", id, body, action);
                                    warp::reply::json(&body)
                                }),
                        )
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| {
                    Err(HttpApiProblem::with_title_and_type_from_status(400)
                        .set_detail("Requested action is not supported for this swap"))
                })
        })
    )
}

trait MatchesAction<A> {
    fn matches(self, action: &A) -> bool;
}

impl<Deploy, Fund, Redeem, Refund> MatchesAction<alice::ActionKind<Deploy, Fund, Redeem, Refund>>
    for GetAction
{
    fn matches(self, other: &alice::ActionKind<Deploy, Fund, Redeem, Refund>) -> bool {
        match other {
            alice::ActionKind::Deploy(_) => self == GetAction::Deploy,
            alice::ActionKind::Fund(_) => self == GetAction::Fund,
            alice::ActionKind::Redeem(_) => self == GetAction::Redeem,
            alice::ActionKind::Refund(_) => self == GetAction::Refund,
        }
    }
}
impl<Accept, Decline, Deploy, Fund, Redeem, Refund>
    MatchesAction<bob::ActionKind<Accept, Decline, Deploy, Fund, Redeem, Refund>> for GetAction
{
    fn matches(
        self,
        other: &bob::ActionKind<Accept, Decline, Deploy, Fund, Redeem, Refund>,
    ) -> bool {
        match other {
            bob::ActionKind::Deploy(_) => self == GetAction::Deploy,
            bob::ActionKind::Fund(_) => self == GetAction::Fund,
            bob::ActionKind::Redeem(_) => self == GetAction::Redeem,
            bob::ActionKind::Refund(_) => self == GetAction::Refund,
            _ => false,
        }
    }
}

impl FromStr for GetAction {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        match s {
            "deploy" => Ok(GetAction::Deploy),
            "fund" => Ok(GetAction::Fund),
            "redeem" => Ok(GetAction::Redeem),
            "refund" => Ok(GetAction::Refund),
            _ => Err(()),
        }
    }
}

pub trait IntoResponseBody {
    fn into_response_body(
        self,
        query_params: GetActionQueryParams,
    ) -> Result<ActionResponseBody, HttpApiProblem>;
}

impl IntoResponseBody for bitcoin::SendToAddress {
    fn into_response_body(
        self,
        query_params: GetActionQueryParams,
    ) -> Result<ActionResponseBody, HttpApiProblem> {
        match query_params {
            GetActionQueryParams::None {} => {
                let bitcoin::SendToAddress {
                    to,
                    amount,
                    network,
                } = self.clone();
                Ok(ActionResponseBody::BitcoinSendAmountToAddress {
                    to,
                    amount,
                    network,
                })
            }
            _ => {
                error!("Unexpected GET parameters for a bitcoin::SendToAddress action type. Expected: none.");
                Err(HttpApiProblem::with_title_and_type_from_status(400)
                    .set_detail("This action does not take any query parameters"))
            }
        }
    }
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

#[derive(Serialize)]
struct MissingQueryParameter {
    data_type: &'static str,
    description: &'static str,
}

impl IntoResponseBody for bitcoin::SpendOutput {
    fn into_response_body(
        self,
        query_params: GetActionQueryParams,
    ) -> Result<ActionResponseBody, HttpApiProblem> {
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
                            return Err(HttpApiProblem::with_title_and_type_from_status(500)
                                .set_detail("Issue encountered when signing Bitcoin transaction"));
                        }
                    };
                    match serialize_hex(&transaction) {
                        Ok(hex) => Ok(ActionResponseBody::BitcoinBroadcastSignedTransaction {
                            hex,
                            network,
                        }),
                        Err(e) => {
                            error!("Could not serialized signed Bitcoin transaction: {:?}", e);
                            Err(
                                HttpApiProblem::with_title_and_type_from_status(500).set_detail(
                                    "Issue encountered when serializing Bitcoin transaction",
                                ),
                            )
                        }
                    }
                }
                Err(_) => Err(HttpApiProblem::with_title_and_type_from_status(400)
                    .set_detail("fee-per-byte is not a valid float")),
            },
            _ => {
                error!("Unexpected GET parameters for a bitcoin::SpendOutput action type. Expected: address and fee-per-byte.");
                let mut problem = HttpApiProblem::with_title_and_type_from_status(400)
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

impl IntoResponseBody for ethereum::ContractDeploy {
    fn into_response_body(
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
            _ => {
                error!("Unexpected GET parameters for an ethereum::ContractDeploy action type. Expected: None.");
                Err(HttpApiProblem::with_title_and_type_from_status(400)
                    .set_detail("This action does not take any query parameters"))
            }
        }
    }
}

impl IntoResponseBody for ethereum::SendTransaction {
    fn into_response_body(
        self,
        query_params: GetActionQueryParams,
    ) -> Result<ActionResponseBody, HttpApiProblem> {
        let ethereum::SendTransaction {
            to,
            data,
            amount,
            gas_limit,
            network,
        } = self;
        match query_params {
            GetActionQueryParams::None {} => Ok(ActionResponseBody::EthereumInvokeContract {
                contract_address: to,
                data,
                amount,
                gas_limit,
                network,
            }),
            _ => {
                error!("Unexpected GET parameters for an ethereum::SendTransaction action. Expected: None.");
                Err(HttpApiProblem::with_title_and_type_from_status(400)
                    .set_detail("This action does not take any query parameters"))
            }
        }
    }
}

impl IntoResponseBody for () {
    fn into_response_body(
        self,
        _: GetActionQueryParams,
    ) -> Result<ActionResponseBody, HttpApiProblem> {
        error!("IntoResponseBody should not be called for the unit type");
        Err(HttpApiProblem::with_title_and_type_from_status(500))
    }
}

impl<Deploy, Fund, Redeem, Refund> IntoResponseBody
    for alice::ActionKind<Deploy, Fund, Redeem, Refund>
where
    Deploy: IntoResponseBody,
    Fund: IntoResponseBody,
    Redeem: IntoResponseBody,
    Refund: IntoResponseBody,
{
    fn into_response_body(
        self,
        query_params: GetActionQueryParams,
    ) -> Result<ActionResponseBody, HttpApiProblem> {
        match self {
            alice::ActionKind::Deploy(payload) => payload.into_response_body(query_params),
            alice::ActionKind::Fund(payload) => payload.into_response_body(query_params),
            alice::ActionKind::Redeem(payload) => payload.into_response_body(query_params),
            alice::ActionKind::Refund(payload) => payload.into_response_body(query_params),
        }
    }
}

impl<Accept, Decline, Deploy, Fund, Redeem, Refund> IntoResponseBody
    for bob::ActionKind<Accept, Decline, Deploy, Fund, Redeem, Refund>
where
    Deploy: IntoResponseBody,
    Fund: IntoResponseBody,
    Redeem: IntoResponseBody,
    Refund: IntoResponseBody,
{
    fn into_response_body(
        self,
        query_params: GetActionQueryParams,
    ) -> Result<ActionResponseBody, HttpApiProblem> {
        match self {
            bob::ActionKind::Deploy(payload) => payload.into_response_body(query_params),
            bob::ActionKind::Fund(payload) => payload.into_response_body(query_params),
            bob::ActionKind::Redeem(payload) => payload.into_response_body(query_params),
            bob::ActionKind::Refund(payload) => payload.into_response_body(query_params),
            _ => {
                error!("IntoResponseBody is not implemented for Accept/Decline");
                Err(HttpApiProblem::with_title_and_type_from_status(500))
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
