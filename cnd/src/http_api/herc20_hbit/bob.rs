use crate::{
    actions::bitcoin::{BroadcastSignedTransaction, SendToAddress},
    http_api::{
        hbit, herc20,
        protocol::{
            AlphaAbsoluteExpiry, AlphaEvents, AlphaLedger, AlphaParams, BetaAbsoluteExpiry,
            BetaEvents, BetaLedger, BetaParams, BobSwap, Hbit, Herc20, Ledger, LedgerEvents,
        },
        ActionNotFound,
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
            } => Some(herc20_state.clone().into()),
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
            } => Some(state.clone().into()),
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
