use crate::{
    actions::bitcoin::BroadcastSignedTransaction,
    http_api::{
        hbit, herc20,
        protocol::{
            AlphaAbsoluteExpiry, AlphaEvents, AlphaLedger, AlphaParams, BetaAbsoluteExpiry,
            BetaEvents, BetaLedger, BetaParams, Hbit, Herc20, Ledger, LedgerEvents,
        },
        ActionNotFound, AliceSwap,
    },
    DeployAction, FundAction, InitAction, RedeemAction, RefundAction, Timestamp,
};
use comit::{actions::ethereum, asset, Never, SecretHash};

impl DeployAction
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsRedeemer>
{
    type Output = ethereum::DeployContract;

    fn deploy_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            AliceSwap::Finalized {
                alpha_finalized:
                    herc20
                    @
                    herc20::Finalized {
                        state: herc20::State::None,
                        ..
                    },
                beta_finalized:
                    hbit::FinalizedAsRedeemer {
                        state: hbit::State::None,
                        ..
                    },
                secret,
                ..
            } => {
                let secret_hash = SecretHash::new(*secret);
                let deploy_action = herc20.build_deploy_action(secret_hash);
                Ok(deploy_action)
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl FundAction
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsRedeemer>
{
    type Output = ethereum::CallContract;

    fn fund_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            AliceSwap::Finalized {
                alpha_finalized:
                    herc20
                    @
                    herc20::Finalized {
                        state: herc20::State::Deployed { .. },
                        ..
                    },
                beta_finalized:
                    hbit::FinalizedAsRedeemer {
                        state: hbit::State::None,
                        ..
                    },
                secret,
                ..
            } => {
                let secret_hash = SecretHash::new(*secret);
                let fund_action = herc20.build_fund_action(secret_hash)?;
                Ok(fund_action)
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl RedeemAction
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsRedeemer>
{
    type Output = BroadcastSignedTransaction;

    fn redeem_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            AliceSwap::Finalized {
                beta_finalized:
                    hbit
                    @
                    hbit::FinalizedAsRedeemer {
                        state: hbit::State::Funded { .. },
                        ..
                    },
                secret,
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
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsRedeemer>
{
    type Output = ethereum::CallContract;

    fn refund_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            AliceSwap::Finalized {
                alpha_finalized:
                    herc20
                    @
                    herc20::Finalized {
                        state: herc20::State::Funded { .. },
                        ..
                    },
                beta_finalized: hbit::FinalizedAsRedeemer { .. },
                secret,
                ..
            } => {
                let secret_hash = SecretHash::new(*secret);
                let refund_action = herc20.build_refund_action(secret_hash)?;
                Ok(refund_action)
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl AlphaParams
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsRedeemer>
{
    type Output = Herc20;
    fn alpha_params(&self) -> Self::Output {
        self.clone().into()
    }
}

impl AlphaEvents
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsRedeemer>
{
    fn alpha_events(&self) -> Option<LedgerEvents> {
        match self {
            AliceSwap::Created { .. } => None,
            AliceSwap::Finalized {
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

impl BetaParams
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsRedeemer>
{
    type Output = Hbit;
    fn beta_params(&self) -> Self::Output {
        self.clone().into()
    }
}

impl BetaEvents
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsRedeemer>
{
    fn beta_events(&self) -> Option<LedgerEvents> {
        match self {
            AliceSwap::Finalized {
                beta_finalized:
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

impl InitAction
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsRedeemer>
{
    type Output = Never;
    fn init_action(&self) -> anyhow::Result<Self::Output> {
        anyhow::bail!(ActionNotFound)
    }
}

impl From<AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsRedeemer>>
    for Herc20
{
    fn from(
        from: AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsRedeemer>,
    ) -> Self {
        match from {
            AliceSwap::Created {
                alpha_created: herc20_asset,
                ..
            }
            | AliceSwap::Finalized {
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

impl From<AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsRedeemer>>
    for Hbit
{
    fn from(
        from: AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsRedeemer>,
    ) -> Self {
        match from {
            AliceSwap::Created {
                beta_created: bitcoin_asset,
                ..
            }
            | AliceSwap::Finalized {
                beta_finalized:
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

impl AlphaLedger
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsRedeemer>
{
    fn alpha_ledger(&self) -> Ledger {
        Ledger::Ethereum
    }
}

impl BetaLedger
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsRedeemer>
{
    fn beta_ledger(&self) -> Ledger {
        Ledger::Bitcoin
    }
}

impl AlphaAbsoluteExpiry
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsRedeemer>
{
    fn alpha_absolute_expiry(&self) -> Option<Timestamp> {
        match self {
            AliceSwap::Created { .. } => None,
            AliceSwap::Finalized {
                alpha_finalized: herc20::Finalized { expiry, .. },
                ..
            } => Some(*expiry),
        }
    }
}

impl BetaAbsoluteExpiry
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsRedeemer>
{
    fn beta_absolute_expiry(&self) -> Option<Timestamp> {
        match self {
            AliceSwap::Created { .. } => None,
            AliceSwap::Finalized {
                beta_finalized: hbit::FinalizedAsRedeemer { expiry, .. },
                ..
            } => Some(*expiry),
        }
    }
}
