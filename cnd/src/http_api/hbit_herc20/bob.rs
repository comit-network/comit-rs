use crate::{
    actions::bitcoin::BroadcastSignedTransaction,
    http_api::{
        hbit, herc20, ActionNotFound, AlphaAbsoluteExpiry, AlphaLedger, AlphaProtocol,
        BetaAbsoluteExpiry, BetaLedger, BetaProtocol, BobSwap, Events, Ledger, Protocol, SwapEvent,
    },
    DeployAction, FundAction, InitAction, RedeemAction, RefundAction, Timestamp,
};
use comit::{actions::ethereum, asset, Never};

impl DeployAction
    for BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>
{
    type Output = ethereum::DeployContract;

    fn deploy_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            BobSwap::Finalized {
                alpha_finalized:
                    hbit::FinalizedAsRedeemer {
                        state: hbit::State::Funded { .. },
                        ..
                    },
                beta_finalized:
                    herc20
                    @
                    herc20::Finalized {
                        state: herc20::State::None,
                        ..
                    },
                secret_hash,
                ..
            } => {
                let deploy_action = herc20.build_deploy_action(*secret_hash);
                Ok(deploy_action)
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl FundAction
    for BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>
{
    type Output = ethereum::CallContract;

    fn fund_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            BobSwap::Finalized {
                alpha_finalized:
                    hbit::FinalizedAsRedeemer {
                        state: hbit::State::Funded { .. },
                        ..
                    },
                beta_finalized:
                    herc20
                    @
                    herc20::Finalized {
                        state: herc20::State::Deployed { .. },
                        ..
                    },
                secret_hash,
                ..
            } => {
                let fund_action = herc20.build_fund_action(*secret_hash)?;
                Ok(fund_action)
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl RedeemAction
    for BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>
{
    type Output = BroadcastSignedTransaction;

    fn redeem_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            BobSwap::Finalized {
                alpha_finalized:
                    hbit
                    @
                    hbit::FinalizedAsRedeemer {
                        state: hbit::State::Funded { .. },
                        ..
                    },
                beta_finalized:
                    herc20::Finalized {
                        state: herc20::State::Redeemed { secret, .. },
                        ..
                    },
                ..
            } => {
                let redeem_action = hbit.build_redeem_action(*secret)?;
                Ok(redeem_action)
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl RefundAction
    for BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>
{
    type Output = ethereum::CallContract;

    fn refund_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            BobSwap::Finalized {
                beta_finalized:
                    herc20
                    @
                    herc20::Finalized {
                        state: herc20::State::Funded { .. },
                        ..
                    },
                secret_hash,
                ..
            } => {
                let refund_action = herc20.build_refund_action(*secret_hash)?;
                Ok(refund_action)
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl AlphaProtocol
    for BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>
{
    fn alpha_protocol(&self) -> Protocol {
        match self {
            BobSwap::Created {
                alpha_created: bitcoin_asset,
                ..
            }
            | BobSwap::Finalized {
                alpha_finalized:
                    hbit::FinalizedAsRedeemer {
                        asset: bitcoin_asset,
                        ..
                    },
                ..
            } => Protocol::hbit(*bitcoin_asset),
        }
    }
}

impl BetaProtocol
    for BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>
{
    fn beta_protocol(&self) -> Protocol {
        match self {
            BobSwap::Created {
                beta_created: herc20_asset,
                ..
            }
            | BobSwap::Finalized {
                beta_finalized:
                    herc20::Finalized {
                        asset: herc20_asset,
                        ..
                    },
                ..
            } => Protocol::herc20_dai(herc20_asset.quantity.clone()),
        }
    }
}

impl Events
    for BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>
{
    fn events(&self) -> Vec<SwapEvent> {
        match self {
            BobSwap::Finalized {
                alpha_finalized:
                    hbit::FinalizedAsRedeemer {
                        state: hbit_state, ..
                    },
                beta_finalized:
                    herc20::Finalized {
                        state: herc20_state,
                        ..
                    },
                ..
            } => {
                let mut events = Vec::new();
                events.extend(Vec::from(herc20_state));
                events.extend(Vec::from(hbit_state));

                events
            }
            _ => Vec::new(),
        }
    }
}

impl InitAction
    for BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>
{
    type Output = Never;
    fn init_action(&self) -> anyhow::Result<Self::Output> {
        anyhow::bail!(ActionNotFound)
    }
}

impl AlphaLedger
    for BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>
{
    fn alpha_ledger(&self) -> Ledger {
        Ledger::Bitcoin
    }
}

impl BetaLedger
    for BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>
{
    fn beta_ledger(&self) -> Ledger {
        Ledger::Ethereum
    }
}

impl AlphaAbsoluteExpiry
    for BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>
{
    fn alpha_absolute_expiry(&self) -> Option<Timestamp> {
        match self {
            BobSwap::Created { .. } => None,
            BobSwap::Finalized {
                alpha_finalized: hbit::FinalizedAsRedeemer { expiry, .. },
                ..
            } => Some(*expiry),
        }
    }
}

impl BetaAbsoluteExpiry
    for BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>
{
    fn beta_absolute_expiry(&self) -> Option<Timestamp> {
        match self {
            BobSwap::Created { .. } => None,
            BobSwap::Finalized {
                beta_finalized: herc20::Finalized { expiry, .. },
                ..
            } => Some(*expiry),
        }
    }
}
