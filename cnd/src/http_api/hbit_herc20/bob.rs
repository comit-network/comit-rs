use crate::{
    actions::bitcoin::BroadcastSignedTransaction,
    http_api::{
        hbit, herc20,
        protocol::{
            AlphaAbsoluteExpiry, AlphaEvents, AlphaLedger, AlphaParams, BetaAbsoluteExpiry,
            BetaEvents, BetaLedger, BetaParams, Hbit, Herc20, Ledger, LedgerEvents,
        },
        ActionNotFound, BobSwap,
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
                ..
            } => {
                let fund_action = herc20.build_fund_action()?;
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
                ..
            } => {
                let refund_action = herc20.build_refund_action()?;
                Ok(refund_action)
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl AlphaParams
    for BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>
{
    type Output = Hbit;
    fn alpha_params(&self) -> Self::Output {
        self.clone().into()
    }
}

impl AlphaEvents
    for BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>
{
    fn alpha_events(&self) -> Option<LedgerEvents> {
        match self {
            BobSwap::Finalized {
                alpha_finalized:
                    hbit::FinalizedAsRedeemer {
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
    for BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>
{
    type Output = Herc20;
    fn beta_params(&self) -> Self::Output {
        self.clone().into()
    }
}

impl BetaEvents
    for BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>
{
    fn beta_events(&self) -> Option<LedgerEvents> {
        match self {
            BobSwap::Finalized {
                beta_finalized:
                    herc20::Finalized {
                        state: herc20_state,
                        ..
                    },
                ..
            } => Some(herc20_state.clone().into()),
            _ => None,
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

impl From<BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>>
    for Hbit
{
    fn from(
        from: BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>,
    ) -> Self {
        match from {
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
            } => Self {
                protocol: "hbit".to_owned(),
                quantity: bitcoin_asset.as_sat().to_string(),
            },
        }
    }
}

impl From<BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>>
    for Herc20
{
    fn from(
        from: BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>,
    ) -> Self {
        match from {
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
            } => Self {
                protocol: "herc20".to_owned(),
                quantity: herc20_asset.quantity.to_wei_dec(),
                token_contract: herc20_asset.token_contract.to_string(),
            },
        }
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
