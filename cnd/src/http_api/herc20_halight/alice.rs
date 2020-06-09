use crate::{
    asset,
    http_api::{
        halight, herc20,
        protocol::{
            AlphaAbsoluteExpiry, AlphaEvents, AlphaLedger, AlphaParams, BetaAbsoluteExpiry,
            BetaEvents, BetaLedger, BetaParams, Halight, Herc20, Ledger, LedgerEvents,
        },
        ActionNotFound, AliceSwap,
    },
    swap_protocols::actions::{ethereum, lnd},
    DeployAction, FundAction, InitAction, RedeemAction, RefundAction, SecretHash, Timestamp,
};

impl From<AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized>>
    for Herc20
{
    fn from(
        from: AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized>,
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

impl From<AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized>>
    for Halight
{
    fn from(
        from: AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized>,
    ) -> Self {
        match from {
            AliceSwap::Created {
                beta_created: halight_asset,
                ..
            }
            | AliceSwap::Finalized {
                beta_finalized:
                    halight::Finalized {
                        asset: halight_asset,
                        ..
                    },
                ..
            } => Self {
                protocol: "halight".to_owned(),
                quantity: halight_asset.as_sat().to_string(),
            },
        }
    }
}

impl AlphaParams
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized>
{
    type Output = Herc20;
    fn alpha_params(&self) -> Self::Output {
        self.clone().into()
    }
}

impl AlphaEvents
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized>
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

impl BetaParams for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized> {
    type Output = Halight;
    fn beta_params(&self) -> Self::Output {
        self.clone().into()
    }
}

impl BetaEvents for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized> {
    fn beta_events(&self) -> Option<LedgerEvents> {
        match self {
            AliceSwap::Created { .. } => None,
            AliceSwap::Finalized {
                beta_finalized:
                    halight::Finalized {
                        state: halight_state,
                        ..
                    },
                ..
            } => Some(halight_state.clone().into()),
        }
    }
}

impl InitAction for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized> {
    type Output = lnd::AddHoldInvoice;

    fn init_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            AliceSwap::Finalized {
                alpha_finalized:
                    herc20::Finalized {
                        state: herc20::State::None,
                        ..
                    },
                beta_finalized:
                    halight
                    @
                    halight::Finalized {
                        state: halight::State::None,
                        ..
                    },
                secret,
                ..
            } => {
                let secret_hash = SecretHash::new(*secret);
                let init_action = halight.build_init_action(secret_hash);
                Ok(init_action)
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl DeployAction
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized>
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
                    halight::Finalized {
                        state: halight::State::Opened(_),
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

impl FundAction for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized> {
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
                    halight::Finalized {
                        state: halight::State::Opened(_),
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
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized>
{
    type Output = lnd::SettleInvoice;

    fn redeem_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            AliceSwap::Finalized {
                beta_finalized:
                    halight
                    @
                    halight::Finalized {
                        state: halight::State::Accepted(_),
                        ..
                    },
                secret,
                ..
            } => {
                let redeem_action = halight.build_redeem_action(*secret);
                Ok(redeem_action)
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl RefundAction
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized>
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
                ..
            } => {
                let refund_action = herc20.build_refund_action()?;
                Ok(refund_action)
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl AlphaLedger
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized>
{
    fn alpha_ledger(&self) -> Ledger {
        Ledger::Ethereum
    }
}

impl BetaLedger for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized> {
    fn beta_ledger(&self) -> Ledger {
        Ledger::Bitcoin
    }
}

impl AlphaAbsoluteExpiry
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized>
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
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized>
{
    fn beta_absolute_expiry(&self) -> Option<Timestamp> {
        None // No absolute expiry time for halight.
    }
}
