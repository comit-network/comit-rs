use crate::{
    actions::{ethereum, lnd},
    asset,
    halbit::Settled,
    http_api::{
        halbit, herc20,
        protocol::{
            AlphaAbsoluteExpiry, AlphaEvents, AlphaLedger, AlphaParams, BetaAbsoluteExpiry,
            BetaEvents, BetaLedger, BetaParams, BobSwap, Halbit, Herc20, Ledger, LedgerEvents,
        },
        ActionNotFound,
    },
    DeployAction, FundAction, InitAction, Never, RedeemAction, RefundAction, Timestamp,
};

impl FundAction for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized> {
    type Output = lnd::SendPayment;

    fn fund_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            BobSwap::Finalized {
                alpha_finalized:
                    herc20::Finalized {
                        state: herc20::State::Funded { .. },
                        ..
                    },
                beta_finalized:
                    halbit
                    @
                    halbit::Finalized {
                        state: halbit::State::Opened(_),
                        ..
                    },
                secret_hash,
            } => {
                let fund_action = halbit.build_fund_action(*secret_hash);
                Ok(fund_action)
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl RedeemAction for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized> {
    type Output = ethereum::CallContract;

    fn redeem_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            BobSwap::Finalized {
                alpha_finalized: herc20 @ herc20::Finalized { .. },
                beta_finalized:
                    halbit::Finalized {
                        state: halbit::State::Settled(Settled { secret }),
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

impl AlphaEvents for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized> {
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

impl BetaEvents for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized> {
    fn beta_events(&self) -> Option<LedgerEvents> {
        match self {
            BobSwap::Created { .. } => None,
            BobSwap::Finalized {
                beta_finalized:
                    halbit::Finalized {
                        state: halbit_state,
                        ..
                    },
                ..
            } => Some(halbit_state.clone().into()),
        }
    }
}

impl AlphaParams for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized> {
    type Output = Herc20;
    fn alpha_params(&self) -> Self::Output {
        self.clone().into()
    }
}

impl BetaParams for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized> {
    type Output = Halbit;
    fn beta_params(&self) -> Self::Output {
        self.clone().into()
    }
}

impl DeployAction for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized> {
    type Output = Never;
    fn deploy_action(&self) -> anyhow::Result<Self::Output> {
        anyhow::bail!(ActionNotFound)
    }
}

impl InitAction for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized> {
    type Output = Never;
    fn init_action(&self) -> anyhow::Result<Self::Output> {
        anyhow::bail!(ActionNotFound)
    }
}

impl RefundAction for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized> {
    type Output = Never;
    fn refund_action(&self) -> anyhow::Result<Self::Output> {
        anyhow::bail!(ActionNotFound)
    }
}

impl From<BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized>> for Herc20 {
    fn from(
        from: BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized>,
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

impl From<BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized>> for Halbit {
    fn from(
        from: BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized>,
    ) -> Self {
        match from {
            BobSwap::Created {
                beta_created: halbit_asset,
                ..
            }
            | BobSwap::Finalized {
                beta_finalized:
                    halbit::Finalized {
                        asset: halbit_asset,
                        ..
                    },
                ..
            } => Self {
                protocol: "halbit".to_owned(),
                quantity: halbit_asset.as_sat().to_string(),
            },
        }
    }
}

impl AlphaLedger for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized> {
    fn alpha_ledger(&self) -> Ledger {
        Ledger::Ethereum
    }
}

impl BetaLedger for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized> {
    fn beta_ledger(&self) -> Ledger {
        Ledger::Bitcoin
    }
}

impl AlphaAbsoluteExpiry
    for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized>
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
    for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized>
{
    fn beta_absolute_expiry(&self) -> Option<Timestamp> {
        None // No absolute expiry time for halbit.
    }
}
