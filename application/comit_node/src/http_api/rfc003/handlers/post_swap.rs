use crate::{
    http_api::{
        problem,
        rfc003::routes::{SwapRequestBody, SwapRequestBodyIdentities, SwapRequestBodyKind},
    },
    swap_protocols::{
        asset::Asset,
        ledger::{Bitcoin, Ethereum},
        rfc003::{
            self,
            alice::{AliceSpawner, SwapRequestIdentities},
            Ledger, SecretSource,
        },
        SwapId,
    },
};
use http_api_problem::HttpApiProblem;

#[derive(Serialize, Debug)]
pub struct SwapCreated {
    pub id: SwapId,
}

trait FromSwapRequestBodyIdentities<AL: Ledger, BL: Ledger>
where
    Self: Sized,
{
    fn from_swap_request_body_identities(
        identities: SwapRequestBodyIdentities<AL::Identity, BL::Identity>,
        id: SwapId,
        secret_source: &dyn SecretSource,
    ) -> Result<Self, HttpApiProblem>;
}

impl FromSwapRequestBodyIdentities<Bitcoin, Ethereum>
    for rfc003::alice::SwapRequestIdentities<Bitcoin, Ethereum>
{
    fn from_swap_request_body_identities(
        identities: SwapRequestBodyIdentities<
            bitcoin_support::PubkeyHash,
            ethereum_support::Address,
        >,
        id: SwapId,
        secret_source: &dyn SecretSource,
    ) -> Result<Self, HttpApiProblem> {
        match identities {
            SwapRequestBodyIdentities::RefundAndRedeem { .. }
            | SwapRequestBodyIdentities::OnlyRefund { .. }
            | SwapRequestBodyIdentities::None {} => {
                Err(HttpApiProblem::with_title_and_type_from_status(400))
            }
            SwapRequestBodyIdentities::OnlyRedeem {
                beta_ledger_redeem_identity,
            } => Ok(rfc003::alice::SwapRequestIdentities {
                alpha_ledger_refund_identity: secret_source.new_secp256k1_refund(id),
                beta_ledger_redeem_identity,
            }),
        }
    }
}

impl FromSwapRequestBodyIdentities<Ethereum, Bitcoin>
    for rfc003::alice::SwapRequestIdentities<Ethereum, Bitcoin>
{
    fn from_swap_request_body_identities(
        identities: SwapRequestBodyIdentities<
            ethereum_support::Address,
            bitcoin_support::PubkeyHash,
        >,
        id: SwapId,
        secret_source: &dyn SecretSource,
    ) -> Result<Self, HttpApiProblem> {
        match identities {
            SwapRequestBodyIdentities::RefundAndRedeem { .. }
            | SwapRequestBodyIdentities::OnlyRedeem { .. }
            | SwapRequestBodyIdentities::None {} => {
                Err(HttpApiProblem::with_title_and_type_from_status(400))
            }
            SwapRequestBodyIdentities::OnlyRefund {
                alpha_ledger_refund_identity,
            } => Ok(rfc003::alice::SwapRequestIdentities {
                alpha_ledger_refund_identity,
                beta_ledger_redeem_identity: secret_source.new_secp256k1_redeem(id),
            }),
        }
    }
}

trait FromSwapRequestBody<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>
where
    Self: Sized,
{
    fn from_swap_request_body(
        body: SwapRequestBody<AL, BL, AA, BA>,
        id: SwapId,
        secret_source: &dyn SecretSource,
    ) -> Result<Self, HttpApiProblem>;
}

impl<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> FromSwapRequestBody<AL, BL, AA, BA>
    for rfc003::alice::SwapRequest<AL, BL, AA, BA>
where
    SwapRequestIdentities<AL, BL>: FromSwapRequestBodyIdentities<AL, BL>,
{
    fn from_swap_request_body(
        body: SwapRequestBody<AL, BL, AA, BA>,
        id: SwapId,
        secret_source: &dyn SecretSource,
    ) -> Result<Self, HttpApiProblem> {
        Ok(rfc003::alice::SwapRequest {
            alpha_asset: body.alpha_asset,
            beta_asset: body.beta_asset,
            alpha_ledger: body.alpha_ledger,
            beta_ledger: body.beta_ledger,
            alpha_expiry: body.alpha_expiry,
            beta_expiry: body.beta_expiry,
            identities: SwapRequestIdentities::from_swap_request_body_identities(
                body.identities,
                id,
                secret_source,
            )?,
            bob_socket_address: body.peer,
        })
    }
}

pub fn handle_post_swap<A: AliceSpawner>(
    alice_spawner: &A,
    secret_source: &dyn SecretSource,
    request_body_kind: SwapRequestBodyKind,
) -> Result<SwapCreated, HttpApiProblem> {
    let id = SwapId::default();

    match request_body_kind {
        SwapRequestBodyKind::BitcoinEthereumBitcoinQuantityErc20Token(body) => alice_spawner
            .spawn(
                id,
                rfc003::alice::SwapRequest::from_swap_request_body(body, id, secret_source)?,
            )?,
        SwapRequestBodyKind::BitcoinEthereumBitcoinQuantityEtherQuantity(body) => alice_spawner
            .spawn(
                id,
                rfc003::alice::SwapRequest::from_swap_request_body(body, id, secret_source)?,
            )?,
        SwapRequestBodyKind::EthereumBitcoinEtherQuantityBitcoinQuantity(body) => alice_spawner
            .spawn(
                id,
                rfc003::alice::SwapRequest::from_swap_request_body(body, id, secret_source)?,
            )?,
        SwapRequestBodyKind::EthereumBitcoinErc20TokenBitcoinQuantity(body) => alice_spawner
            .spawn(
                id,
                rfc003::alice::SwapRequest::from_swap_request_body(body, id, secret_source)?,
            )?,
        SwapRequestBodyKind::UnsupportedCombination(body) => {
            error!(
                "Swapping {:?} for {:?} from {:?} to {:?} is not supported",
                body.alpha_asset, body.beta_asset, body.alpha_ledger, body.beta_ledger
            );
            return Err(problem::unsupported());
        }
        SwapRequestBodyKind::MalformedRequest(body) => {
            error!(
                "Malformed request body: {}",
                serde_json::to_string(&body)
                    .expect("failed to serialize serde_json::Value as string ?!")
            );
            return Err(HttpApiProblem::with_title_and_type_from_status(400)
                .set_detail("The request body was malformed"));
        }
    };

    Ok(SwapCreated { id })
}
