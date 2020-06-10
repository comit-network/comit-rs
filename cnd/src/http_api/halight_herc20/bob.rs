use crate::{
    asset,
    http_api::{
        halight, herc20,
        protocol::{
            AlphaAbsoluteExpiry, AlphaEvents, AlphaLedger, AlphaParams, BetaAbsoluteExpiry,
            BetaEvents, BetaLedger, BetaParams, Halight, Herc20, Ledger, LedgerEvents,
        },
        ActionNotFound, BobSwap,
    },
    swap_protocols::actions::{ethereum, lnd},
    DeployAction, FundAction, InitAction, RedeemAction, RefundAction, Timestamp,
};

impl InitAction for BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> {
    type Output = lnd::AddHoldInvoice;

    fn init_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            BobSwap::Finalized {
                alpha_finalized:
                    halight
                    @
                    halight::Finalized {
                        state: halight::State::None,
                        ..
                    },
                secret_hash,
                ..
            } => {
                let init_action = halight.build_init_action(*secret_hash);
                Ok(init_action)
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl DeployAction for BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> {
    type Output = ethereum::DeployContract;

    fn deploy_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            BobSwap::Finalized {
                alpha_finalized:
                    halight::Finalized {
                        state: halight::State::Accepted(_),
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

impl FundAction for BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> {
    type Output = ethereum::CallContract;

    fn fund_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            BobSwap::Finalized {
                alpha_finalized:
                    halight::Finalized {
                        state: halight::State::Accepted(_),
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

impl RedeemAction for BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> {
    type Output = lnd::SettleInvoice;

    fn redeem_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            BobSwap::Finalized {
                alpha_finalized:
                    halight
                    @
                    halight::Finalized {
                        state: halight::State::Accepted(_),
                        ..
                    },
                beta_finalized:
                    herc20::Finalized {
                        state: herc20::State::Redeemed { secret, .. },
                        ..
                    },
                ..
            } => {
                let redeem_action = halight.build_redeem_action(*secret);
                Ok(redeem_action)
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl RefundAction for BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> {
    type Output = ethereum::CallContract;

    fn refund_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            BobSwap::Finalized {
                alpha_finalized:
                    halight::Finalized {
                        state: halight::State::Accepted(_),
                        ..
                    },
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

impl AlphaEvents for BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> {
    fn alpha_events(&self) -> Option<LedgerEvents> {
        match self {
            BobSwap::Created { .. } => None,
            BobSwap::Finalized {
                alpha_finalized:
                    halight::Finalized {
                        state: halight_state,
                        ..
                    },
                ..
            } => Some((*halight_state).into()),
        }
    }
}

impl BetaEvents for BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> {
    fn beta_events(&self) -> Option<LedgerEvents> {
        match self {
            BobSwap::Created { .. } => None,
            BobSwap::Finalized {
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

impl AlphaParams for BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> {
    type Output = Halight;
    fn alpha_params(&self) -> Self::Output {
        self.clone().into()
    }
}

impl BetaParams for BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> {
    type Output = Herc20;
    fn beta_params(&self) -> Self::Output {
        self.clone().into()
    }
}

impl From<BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>>
    for Halight
{
    fn from(
        from: BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>,
    ) -> Self {
        match from {
            BobSwap::Created {
                alpha_created: halight_asset,
                ..
            }
            | BobSwap::Finalized {
                alpha_finalized:
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

impl From<BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>> for Herc20 {
    fn from(
        from: BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>,
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

impl AlphaLedger for BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> {
    fn alpha_ledger(&self) -> Ledger {
        Ledger::Bitcoin
    }
}

impl BetaLedger for BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> {
    fn beta_ledger(&self) -> Ledger {
        Ledger::Ethereum
    }
}

impl AlphaAbsoluteExpiry
    for BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>
{
    fn alpha_absolute_expiry(&self) -> Option<Timestamp> {
        None // No absolute expiry time for halight.
    }
}

impl BetaAbsoluteExpiry
    for BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>
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
