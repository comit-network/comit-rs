use crate::{
    actions::bitcoin::{
        sign_with_fixed_rate, BroadcastSignedTransaction, SendToAddress, SpendOutput,
    },
    http_api::{
        hbit, herc20,
        protocol::{
            AlphaAbsoluteExpiry, AlphaEvents, AlphaLedger, AlphaParams, BetaAbsoluteExpiry,
            BetaEvents, BetaLedger, BetaParams, Hbit, Herc20, Ledger, LedgerEvents,
        },
        ActionNotFound, AliceSwap,
    },
    identity, DeployAction, FundAction, InitAction, RedeemAction, RefundAction, Timestamp,
};
use blockchain_contracts::ethereum::rfc003::EtherHtlc;
use comit::{
    actions::ethereum, asset, ethereum::Bytes, hbit::build_bitcoin_htlc, Never, SecretHash,
};

impl FundAction
    for AliceSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsFunder, herc20::Finalized>
{
    type Output = SendToAddress;

    fn fund_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            AliceSwap::Finalized {
                alpha_finalized:
                    hbit::FinalizedAsFunder {
                        asset,
                        network,
                        transient_redeem_identity: redeem_identity,
                        transient_refund_identity: transient_refund_sk,
                        expiry,
                        state: hbit::State::None,
                        ..
                    },
                secret,
                ..
            } => {
                let refund_identity =
                    identity::Bitcoin::from_secret_key(&*crate::SECP, &transient_refund_sk);
                let htlc = build_bitcoin_htlc(
                    *redeem_identity,
                    refund_identity,
                    *expiry,
                    SecretHash::new(*secret),
                );
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
    for AliceSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsFunder, herc20::Finalized>
{
    type Output = ethereum::CallContract;

    fn redeem_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            AliceSwap::Finalized {
                beta_finalized:
                    herc20::Finalized {
                        chain_id,
                        state: herc20::State::Funded { htlc_location, .. },
                        ..
                    },
                secret,
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
    for AliceSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsFunder, herc20::Finalized>
{
    type Output = BroadcastSignedTransaction;

    fn refund_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            AliceSwap::Finalized {
                alpha_finalized:
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
                secret,
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
                        SecretHash::new(*secret),
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

impl AlphaParams
    for AliceSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsFunder, herc20::Finalized>
{
    type Output = Hbit;
    fn alpha_params(&self) -> Self::Output {
        self.clone().into()
    }
}

impl AlphaEvents
    for AliceSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsFunder, herc20::Finalized>
{
    fn alpha_events(&self) -> Option<LedgerEvents> {
        match self {
            AliceSwap::Finalized {
                alpha_finalized:
                    hbit::FinalizedAsFunder {
                        state: bitcoin_state,
                        ..
                    },
                ..
            } => Some(bitcoin_state.clone().into()),
            _ => None,
        }
    }
}

impl BetaParams
    for AliceSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsFunder, herc20::Finalized>
{
    type Output = Herc20;
    fn beta_params(&self) -> Self::Output {
        self.clone().into()
    }
}

impl BetaEvents
    for AliceSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsFunder, herc20::Finalized>
{
    fn beta_events(&self) -> Option<LedgerEvents> {
        match self {
            AliceSwap::Created { .. } => None,
            AliceSwap::Finalized {
                beta_finalized:
                    herc20::Finalized {
                        state: herc20_state,
                        ..
                    },
                ..
            } => Some(herc20_state.clone().into()),
        }
    }
}

impl InitAction
    for AliceSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsFunder, herc20::Finalized>
{
    type Output = Never;
    fn init_action(&self) -> anyhow::Result<Self::Output> {
        anyhow::bail!(ActionNotFound)
    }
}

impl DeployAction
    for AliceSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsFunder, herc20::Finalized>
{
    type Output = Never;

    fn deploy_action(&self) -> anyhow::Result<Self::Output> {
        anyhow::bail!(ActionNotFound)
    }
}

impl From<AliceSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsFunder, herc20::Finalized>>
    for Hbit
{
    fn from(
        from: AliceSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsFunder, herc20::Finalized>,
    ) -> Self {
        match from {
            AliceSwap::Created {
                alpha_created: bitcoin_asset,
                ..
            }
            | AliceSwap::Finalized {
                alpha_finalized:
                    hbit::FinalizedAsFunder {
                        asset: bitcoin_asset,
                        ..
                    },
                ..
            } => Self {
                protocol: "hbit".to_owned(),
                quantity: bitcoin_asset.as_sat().to_string(),
            },
        }
    }
}

impl From<AliceSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsFunder, herc20::Finalized>>
    for Herc20
{
    fn from(
        from: AliceSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsFunder, herc20::Finalized>,
    ) -> Self {
        match from {
            AliceSwap::Created {
                beta_created: herc20_asset,
                ..
            }
            | AliceSwap::Finalized {
                beta_finalized:
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

impl AlphaLedger
    for AliceSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsFunder, herc20::Finalized>
{
    fn alpha_ledger(&self) -> Ledger {
        Ledger::Bitcoin
    }
}

impl BetaLedger
    for AliceSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsFunder, herc20::Finalized>
{
    fn beta_ledger(&self) -> Ledger {
        Ledger::Ethereum
    }
}

impl AlphaAbsoluteExpiry
    for AliceSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsFunder, herc20::Finalized>
{
    fn alpha_absolute_expiry(&self) -> Option<Timestamp> {
        match self {
            AliceSwap::Created { .. } => None,
            AliceSwap::Finalized {
                alpha_finalized: hbit::FinalizedAsFunder { expiry, .. },
                ..
            } => Some(*expiry),
        }
    }
}

impl BetaAbsoluteExpiry
    for AliceSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsFunder, herc20::Finalized>
{
    fn beta_absolute_expiry(&self) -> Option<Timestamp> {
        match self {
            AliceSwap::Created { .. } => None,
            AliceSwap::Finalized {
                beta_finalized: herc20::Finalized { expiry, .. },
                ..
            } => Some(*expiry),
        }
    }
}
