use crate::{
    actions::bitcoin::{BroadcastSignedTransaction, SendToAddress},
    http_api::{
        hbit, herc20, ActionNotFound, AlphaAbsoluteExpiry, AlphaLedger, AlphaProtocol,
        BetaAbsoluteExpiry, BetaLedger, BetaProtocol, BobSwap, Events, Ledger, Protocol, SwapEvent,
    },
    DeployAction, FundAction, InitAction, RedeemAction, RefundAction, Timestamp,
};
use comit::{actions::ethereum, asset, Never};

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
                    hbit
                    @
                    hbit::FinalizedAsFunder {
                        state: hbit::State::None,
                        ..
                    },
                secret_hash,
            } => {
                let fund_action = hbit.build_fund_action(*secret_hash);
                Ok(fund_action)
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
                    herc20
                    @
                    herc20::Finalized {
                        state: herc20::State::Funded { .. },
                        ..
                    },
                beta_finalized:
                    hbit::FinalizedAsFunder {
                        state: hbit::State::Redeemed { secret, .. },
                        ..
                    },
                ..
            } => {
                let redeem_action = herc20.build_redeem_action(*secret)?;
                Ok(redeem_action)
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
                    hbit
                    @
                    hbit::FinalizedAsFunder {
                        state: hbit::State::Funded { .. },
                        ..
                    },
                secret_hash,
                ..
            } => {
                let refund_action = hbit.build_refund_action(*secret_hash)?;
                Ok(refund_action)
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl Events for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsFunder> {
    fn events(&self) -> Vec<SwapEvent> {
        match self {
            BobSwap::Created { .. } => Vec::new(),
            BobSwap::Finalized {
                alpha_finalized:
                    herc20::Finalized {
                        state: herc20_state,
                        ..
                    },
                beta_finalized:
                    hbit::FinalizedAsFunder {
                        state: hbit_state, ..
                    },
                ..
            } => {
                let mut events = Vec::new();
                events.extend(Vec::from(herc20_state));
                events.extend(Vec::from(hbit_state));

                events
            }
        }
    }
}

impl AlphaProtocol
    for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsFunder>
{
    fn alpha_protocol(&self) -> Protocol {
        match self {
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
            } => Protocol::herc20_dai(herc20_asset.quantity.clone()),
        }
    }
}

impl BetaProtocol
    for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsFunder>
{
    fn beta_protocol(&self) -> Protocol {
        match self {
            BobSwap::Created {
                beta_created: asset,
                ..
            }
            | BobSwap::Finalized {
                beta_finalized: hbit::FinalizedAsFunder { asset, .. },
                ..
            } => Protocol::hbit(*asset),
        }
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
            BobSwap::Created { .. } => None,
            BobSwap::Finalized {
                alpha_finalized: herc20::Finalized { expiry, .. },
                ..
            } => Some(*expiry),
        }
    }
}

impl BetaAbsoluteExpiry
    for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsFunder>
{
    fn beta_absolute_expiry(&self) -> Option<Timestamp> {
        match self {
            BobSwap::Created { .. } => None,
            BobSwap::Finalized {
                beta_finalized: hbit::FinalizedAsFunder { expiry, .. },
                ..
            } => Some(*expiry),
        }
    }
}
