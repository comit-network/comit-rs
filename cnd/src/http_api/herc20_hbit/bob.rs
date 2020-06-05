use crate::{
    actions::bitcoin::{
        sign_with_fixed_rate, BroadcastSignedTransaction, SendToAddress, SpendOutput,
    },
    http_api::{
        hbit, herc20,
        protocol::{
            AlphaAbsoluteExpiry, AlphaEvents, AlphaLedger, AlphaParams, BetaAbsoluteExpiry,
            BetaEvents, BetaLedger, BetaParams, BobSwap, Hbit, Herc20, Ledger, LedgerEvents,
        },
        ActionNotFound,
    },
    identity, DeployAction, FundAction, InitAction, RedeemAction, RefundAction, Timestamp,
};
use blockchain_contracts::ethereum::rfc003::EtherHtlc;
use comit::{actions::ethereum, asset, ethereum::Bytes, Never};
use hbit::build_bitcoin_htlc;

impl FundAction
    for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsFunder>
{
    type Output = SendToAddress;

    fn fund_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            BobSwap::Finalized {
                alpha_finalized:
                    herc20::Finalized {
                        state: herc20::State::Funded { .. },
                        ..
                    },
                beta_finalized:
                    hbit::FinalizedAsFunder {
                        asset,
                        network,
                        transient_redeem_identity: redeem_identity,
                        transient_refund_identity: transient_refund_sk,
                        expiry,
                        state: hbit::State::None,
                        ..
                    },
                secret_hash,
            } => {
                let refund_identity =
                    identity::Bitcoin::from_secret_key(&*crate::SECP, &transient_refund_sk);
                let htlc =
                    build_bitcoin_htlc(*redeem_identity, refund_identity, *expiry, *secret_hash);
                let network = bitcoin::Network::from(*network);
                let to = htlc.compute_address(network);
                let amount = *asset;

                Ok(SendToAddress {
                    to,
                    amount,
                    network,
                })
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl RedeemAction
    for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsFunder>
{
    type Output = ethereum::CallContract;

    fn redeem_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            BobSwap::Finalized {
                alpha_finalized:
                    herc20::Finalized {
                        chain_id,
                        state: herc20::State::Funded { htlc_location, .. },
                        ..
                    },
                beta_finalized:
                    hbit::FinalizedAsFunder {
                        state: hbit::State::Redeemed { secret, .. },
                        ..
                    },
                ..
            } => {
                let to = *htlc_location;
                let data = Some(Bytes::from(secret.into_raw_secret().to_vec()));
                let gas_limit = EtherHtlc::redeem_tx_gas_limit();
                let min_block_timestamp = None;

                Ok(ethereum::CallContract {
                    to,
                    data,
                    gas_limit,
                    chain_id: *chain_id,
                    min_block_timestamp,
                })
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl RefundAction
    for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsFunder>
{
    type Output = BroadcastSignedTransaction;
    fn refund_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            BobSwap::Finalized {
                beta_finalized:
                    hbit::FinalizedAsFunder {
                        network,
                        transient_redeem_identity,
                        transient_refund_identity: transient_refund_sk,
                        final_refund_identity,
                        expiry,
                        state:
                            hbit::State::Funded {
                                htlc_location,
                                fund_transaction,
                                ..
                            },
                        ..
                    },
                secret_hash,
                ..
            } => {
                let network = bitcoin::Network::from(*network);
                let spend_output = {
                    let transient_refund_identity =
                        identity::Bitcoin::from_secret_key(&*crate::SECP, &transient_refund_sk);
                    let htlc = build_bitcoin_htlc(
                        *transient_redeem_identity,
                        transient_refund_identity,
                        *expiry,
                        *secret_hash,
                    );

                    let previous_output = *htlc_location;
                    let value = bitcoin::Amount::from_sat(
                        fund_transaction.output[htlc_location.vout as usize].value,
                    );
                    let input_parameters =
                        htlc.unlock_after_timeout(&*crate::SECP, *transient_refund_sk);

                    SpendOutput::new(previous_output, value, input_parameters, network)
                };

                let primed_transaction =
                    spend_output.spend_to(final_refund_identity.clone().into());
                let transaction = sign_with_fixed_rate(&*crate::SECP, primed_transaction)?;

                Ok(BroadcastSignedTransaction {
                    transaction,
                    network,
                })
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl AlphaEvents
    for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsFunder>
{
    fn alpha_events(&self) -> Option<LedgerEvents> {
        match self {
            BobSwap::Created { .. } => None,
            BobSwap::Finalized {
                alpha_finalized:
                    herc20::Finalized {
                        state: herc20_state,
                        ..
                    },
                ..
            } => Some(From::<herc20::State>::from(herc20_state.clone())),
        }
    }
}

impl BetaEvents
    for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsFunder>
{
    fn beta_events(&self) -> Option<LedgerEvents> {
        match self {
            BobSwap::Created { .. } => None,
            BobSwap::Finalized {
                beta_finalized: hbit::FinalizedAsFunder { state, .. },
                ..
            } => Some(From::<hbit::State>::from(state.clone())),
        }
    }
}

impl AlphaParams
    for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsFunder>
{
    type Output = Herc20;
    fn alpha_params(&self) -> Self::Output {
        self.clone().into()
    }
}

impl BetaParams
    for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsFunder>
{
    type Output = Hbit;
    fn beta_params(&self) -> Self::Output {
        self.clone().into()
    }
}

impl DeployAction
    for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsFunder>
{
    type Output = Never;
    fn deploy_action(&self) -> anyhow::Result<Self::Output> {
        anyhow::bail!(ActionNotFound)
    }
}

impl InitAction
    for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsFunder>
{
    type Output = Never;
    fn init_action(&self) -> anyhow::Result<Self::Output> {
        anyhow::bail!(ActionNotFound)
    }
}

impl From<BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsFunder>>
    for Herc20
{
    fn from(
        from: BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsFunder>,
    ) -> Self {
        match from {
            BobSwap::Created {
                alpha_created: herc20_asset,
                ..
            }
            | BobSwap::Finalized {
                alpha_finalized:
                    herc20::Finalized {
                        asset: herc20_asset,
                        ..
                    },
                ..
            } => Self {
                protocol: "herc20".to_owned(),
                quantity: herc20_asset.quantity.to_wei_dec(),
                token_contract: herc20_asset.token_contract.to_string(),
            },
        }
    }
}

impl From<BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsFunder>>
    for Hbit
{
    fn from(
        from: BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsFunder>,
    ) -> Self {
        match from {
            BobSwap::Created {
                beta_created: asset,
                ..
            }
            | BobSwap::Finalized {
                beta_finalized: hbit::FinalizedAsFunder { asset, .. },
                ..
            } => Self {
                protocol: "hbit".to_owned(),
                quantity: asset.as_sat().to_string(),
            },
        }
    }
}

impl AlphaLedger
    for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsFunder>
{
    fn alpha_ledger(&self) -> Ledger {
        Ledger::Ethereum
    }
}

impl BetaLedger
    for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsFunder>
{
    fn beta_ledger(&self) -> Ledger {
        Ledger::Bitcoin
    }
}

impl AlphaAbsoluteExpiry
    for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsFunder>
{
    fn alpha_absolute_expiry(&self) -> Option<Timestamp> {
        match self {
            BobSwap::<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsFunder>::Created { .. } => None,
            BobSwap::<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsFunder>::Finalized {
                alpha_finalized: herc20::Finalized { expiry, .. },
                ..
            } => Some(*expiry)
        }
    }
}

impl BetaAbsoluteExpiry
    for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsFunder>
{
    fn beta_absolute_expiry(&self) -> Option<Timestamp> {
        match self {
            BobSwap::<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsFunder>::Created { .. } => None,
            BobSwap::<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsFunder>::Finalized {
                beta_finalized: hbit::FinalizedAsFunder { expiry, .. },
                ..
            } => Some(*expiry)
        }
    }
}
